#![allow(clippy::arc_with_non_send_sync)]

use super::{
    context::Context,
    frame::DrawFrame,
    offscreen::OffscreenSurfaceData,
    surface::{Surface, SurfaceCandidate, SurfaceOwner, SurfaceSource},
    utils::{wgpu_depth_stencil, wgpu_shader_visibility},
};
use crate::{
    backend::{
        traits::GfxBackendImpl,
        wgpu::pipeline::{PipelineInner, PipelineRecipe},
    },
    gfx::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutRef, BindType, Buffer,
        BufferDescriptor, BufferUsage, Color, GpuStats, InnerBuffer, Limits, MAX_BINDING_ENTRIES,
        RenderCommand, RenderPass, RenderPipeline, RenderPipelineDescriptor, RenderTexture,
        RenderTextureDescriptor, Renderer, Sampler, SamplerDescriptor, Scissor, Stencil, Texture,
        TextureData, TextureDescriptor, TextureFormat, TextureId,
        consts::{
            MAX_BIND_GROUPS_PER_PIPELINE, MAX_PIPELINE_COMPATIBLE_TEXTURES,
            SURFACE_DEFAULT_DEPTH_FORMAT,
        },
    },
    math::{UVec2, vec2},
};
use arrayvec::ArrayVec;
use atomic_refcell::AtomicRefCell;
#[cfg(target_arch = "wasm32")]
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};
use std::{
    borrow::Cow,
    sync::{Arc, atomic::AtomicU32},
};
use wgpu::{
    BackendOptions, Backends, BufferDescriptor as WBufferDescriptor, Dx12BackendOptions, Extent3d,
    GlBackendOptions, InstanceDescriptor, InstanceFlags, Origin3d, Queue, StoreOp,
    TexelCopyBufferLayout, TextureDimension,
    util::{BufferInitDescriptor, DeviceExt, new_instance_with_webgpu_detection},
};

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
struct WebDisplay;

#[cfg(target_arch = "wasm32")]
impl HasDisplayHandle for WebDisplay {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::web())
    }
}

fn resolve_scissor(scissor: Scissor, width: u32, height: u32) -> Result<[u32; 4], String> {
    match scissor {
        Scissor::Physical(rect) => Ok(rect),
        Scissor::Normalized(edges) => {
            if !edges.iter().all(|edge| edge.is_finite()) {
                return Err("Normalized scissor edges must be finite".to_string());
            }

            let [min_x, min_y, max_x, max_y] = edges;
            if max_x <= min_x || max_y <= min_y {
                return Err("Normalized scissor must have positive area".to_string());
            }

            let min_x = (min_x * width as f32).floor().clamp(0.0, width as f32) as u32;
            let min_y = (min_y * height as f32).floor().clamp(0.0, height as f32) as u32;
            let max_x = (max_x * width as f32).ceil().clamp(0.0, width as f32) as u32;
            let max_y = (max_y * height as f32).ceil().clamp(0.0, height as f32) as u32;
            if max_x <= min_x || max_y <= min_y {
                return Err("Normalized scissor does not intersect the render target".to_string());
            }

            Ok([
                min_x,
                min_y,
                max_x.saturating_sub(min_x),
                max_y.saturating_sub(min_y),
            ])
        }
    }
}

fn pass_uses_depth_stencil(pass: &RenderPass<'_>) -> Result<(bool, bool), String> {
    let has_clear_depth = pass.clear_options.depth.is_some();
    let uses_depth = has_clear_depth
        || pass.commands.iter().any(|command| {
            command
                .pipeline
                .is_some_and(|pipeline| pipeline.inner.uses_depth)
        });
    let has_clear_stencil = pass.clear_options.stencil.is_some();
    let uses_stencil = has_clear_stencil
        || pass.commands.iter().any(|command| {
            command
                .pipeline
                .is_some_and(|pipeline| pipeline.inner.uses_stencil)
        });
    let has_attachment = uses_depth || uses_stencil;
    let has_incompatible_pipeline = has_attachment
        && pass.commands.iter().any(|command| {
            command
                .pipeline
                .is_some_and(|pipeline| !pipeline.inner.uses_depth && !pipeline.inner.uses_stencil)
        });
    if has_incompatible_pipeline {
        return Err(
            "A render pass cannot mix pipelines with and without depth-stencil attachments"
                .to_string(),
        );
    }
    Ok((uses_depth, uses_stencil))
}

fn encode_render_commands(
    pass: &mut wgpu::RenderPass<'_>,
    commands: &[RenderCommand<'_>],
    width: u32,
    height: u32,
) -> Result<(), String> {
    for command in commands {
        let Some(pipeline) = command.pipeline else {
            continue;
        };
        if command
            .vertices
            .iter()
            .all(|vertices| vertices.range.is_empty())
        {
            continue;
        }

        let expected_vertex_buffers = pipeline.inner.recipe.vertex_layout.len();
        let vertex_buffers = command
            .buffers
            .iter()
            .filter(|(buffer, _)| matches!(buffer.usage, BufferUsage::Vertex))
            .count();
        if vertex_buffers != expected_vertex_buffers {
            return Err(format!(
                "Render pipeline requires {expected_vertex_buffers} vertex buffers, but the command binds {vertex_buffers}"
            ));
        }
        if command.bind_groups.len() != pipeline.inner.bind_group_layout.len() {
            return Err(format!(
                "Render pipeline requires {} bind groups, but the command binds {}",
                pipeline.inner.bind_group_layout.len(),
                command.bind_groups.len()
            ));
        }

        pass.set_pipeline(&pipeline.inner.raw);
        let viewport = command.size.unwrap_or(vec2(width as f32, height as f32));
        if !viewport.is_finite() || viewport.x <= 0.0 || viewport.y <= 0.0 {
            return Err("Render command viewport size must be finite and positive".to_string());
        }
        let viewport = viewport.min(vec2(width as f32, height as f32));
        pass.set_viewport(0.0, 0.0, viewport.x, viewport.y, 0.0, 1.0);

        let [x, y, scissor_width, scissor_height] = match command.scissors {
            Some(scissor) => resolve_scissor(scissor, width, height)?,
            None => [0, 0, width, height],
        };
        pass.set_scissor_rect(x, y, scissor_width, scissor_height);
        pass.set_stencil_reference(u32::from(command.stencil_ref.unwrap_or(0)));

        let mut vertex_slot = 0;
        let mut indexed = false;
        for (buffer, range) in &command.buffers {
            if range.is_empty() {
                return Err(format!(
                    "Buffer '{} - ({:?})' range cannot be empty",
                    buffer.inner_label, buffer.id
                ));
            }

            match buffer.usage {
                BufferUsage::Vertex => {
                    pass.set_vertex_buffer(
                        vertex_slot,
                        buffer.inner.borrow().raw.slice(range.clone()),
                    );
                    vertex_slot += 1;
                }
                BufferUsage::Index => {
                    debug_assert!(!indexed, "Cannot bind more than one Index buffer");
                    indexed = true;
                    pass.set_index_buffer(
                        buffer.inner.borrow().raw.slice(range.clone()),
                        pipeline.inner.index_format,
                    );
                }
                BufferUsage::Uniform => {}
            }
        }

        for (index, bind_group) in command.bind_groups.iter().enumerate() {
            let index = u32::try_from(index)
                .map_err(|_| "Bind group index exceeds the supported range".to_string())?;
            pass.set_bind_group(index, &*bind_group.raw, &[]);
        }

        for vertices in &command.vertices {
            if vertices.range.is_empty() {
                continue;
            }
            let instances = 0..vertices.instances.unwrap_or(1);
            if indexed {
                pass.draw_indexed(vertices.range.clone(), 0, instances);
            } else {
                pass.draw(vertices.range.clone(), instances);
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_wine() -> bool {
    use windows::{
        Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress},
        core::s,
    };

    // SAFETY: This only queries the handle for a system module already loaded by the process.
    let Ok(ntdll) = (unsafe { GetModuleHandleA(s!("ntdll.dll")) }) else {
        return false;
    };

    // SAFETY: This only checks whether the export exists; the returned pointer is not retained or called.
    unsafe { GetProcAddress(ntdll, s!("wine_get_version")) }.is_some()
}

#[cfg(target_arch = "wasm32")]
async fn init_wasm_gfx(
    source: SurfaceSource,
    vsync: bool,
    win_size: UVec2,
    pixelated: bool,
    backend_override: Option<Backends>,
) -> Result<GfxBackend, String> {
    let backends = backend_override.unwrap_or_default();
    match GfxBackend::new(source.clone(), vsync, win_size, pixelated, backends).await {
        Ok(gfx) => Ok(gfx),
        #[cfg(feature = "webgl")]
        Err(error) => {
            log::error!("Error initializing Gfx backend: {error}");
            log::info!("Fallback to WebGL");
            GfxBackend::new(source, vsync, win_size, pixelated, Backends::GL).await
        }
        #[cfg(not(feature = "webgl"))]
        Err(error) => {
            log::error!("Error initializing Gfx backend: {error}");
            Err(error)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn init_native_gfx(
    source: SurfaceSource,
    vsync: bool,
    win_size: UVec2,
    pixelated: bool,
    backend_override: Option<Backends>,
) -> Result<GfxBackend, String> {
    let backends = match backend_override {
        Some(backends) => {
            log::info!("Using explicit graphics backend mask: {backends:?}");
            backends
        }
        None => {
            #[cfg(target_os = "windows")]
            {
                if is_wine() {
                    log::info!(
                        "Wine-compatible runtime detected; using default graphics backend selection"
                    );
                    Backends::default()
                } else {
                    log::info!("Using ordered Windows graphics backend policy");
                    let attempts = [Backends::DX12, Backends::VULKAN, Backends::GL];
                    let mut errors = Vec::with_capacity(attempts.len());

                    for backends in attempts {
                        log::info!("Attempting graphics backend: {backends:?}");
                        match GfxBackend::new(source.clone(), vsync, win_size, pixelated, backends)
                            .await
                        {
                            Ok(gfx) => {
                                log::info!(
                                    "Successfully initialized graphics backend: {backends:?}"
                                );
                                return Ok(gfx);
                            }
                            Err(error) => {
                                log::warn!(
                                    "Graphics backend initialization failed: {backends:?}: {error}"
                                );
                                errors.push(format!("{backends:?}: {error}"));
                            }
                        }
                    }

                    let error = format!(
                        "All ordered Windows graphics backend attempts failed:\n{}",
                        errors.join("\n")
                    );
                    log::error!("Error initializing Gfx backend: {error}");
                    return Err(error);
                }
            }

            #[cfg(not(target_os = "windows"))]
            {
                Backends::default()
            }
        }
    };

    GfxBackend::new(source, vsync, win_size, pixelated, backends)
        .await
        .map_err(|error| {
            log::error!("Error initializing Gfx backend: {error}");
            error
        })
}

pub(crate) struct GfxBackend {
    pub(crate) surface: Surface, // Eventually we could have a HashMap<WindowId, Surface> if we want multiple window

    next_resource_id: u64,
    ctx: Context,

    #[cfg(native_windowed)]
    depth_format: TextureFormat,
    frame: Option<DrawFrame>,

    // used as intermediate for surface and pipeline texture formats
    offscreen: Option<OffscreenSurfaceData>,

    last_frame_stats: GpuStats,
    current_stats: GpuStats,
}

// This is a hack for wasm32 browsers where there is no threads
#[cfg(target_arch = "wasm32")]
unsafe impl Send for GfxBackend {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for GfxBackend {}

impl GfxBackendImpl for GfxBackend {
    fn prepare_frame(&mut self) -> Result<(), String> {
        let can_render = self.surface.config.width > 0 && self.surface.config.height > 0;
        if !can_render {
            // on win_os minized windows can report 0 size, skip rendering
            return Ok(());
        }

        self.push_frame()
    }

    fn present_frame(&mut self) {
        self.present_to_screen();

        // new stats
        self.last_frame_stats = std::mem::take(&mut self.current_stats);
    }

    fn render(&mut self, renderer: &Renderer) -> Result<(), String> {
        // TODO change this, "take" is ugly as hell
        let offscreen = self
            .offscreen
            .take()
            .ok_or_else(|| "Invalid Offscreen surface".to_string())
            .unwrap();

        let can_render = offscreen.texture.size.x > 0.0 && offscreen.texture.size.y > 0.0;
        if !can_render {
            // if minimized on windows just skip
            return Ok(());
        }

        self.render_to(&offscreen.texture, renderer)?;
        self.offscreen = Some(offscreen);

        if !renderer.passes.is_empty()
            && let Some(frame) = &mut self.frame
        {
            frame.dirty = true;
        }

        Ok(())
    }

    fn render_to(&mut self, texture: &RenderTexture, renderer: &Renderer) -> Result<(), String> {
        debug_assert!(
            texture.texture.write,
            "Cannot write data to a static render texture"
        );

        if texture.size.x <= 0.0 || texture.size.y <= 0.0 {
            return Ok(());
        }

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RenderTexture Encoder"),
            });

        for pass in &renderer.passes {
            let (uses_depth, uses_stencil) = pass_uses_depth_stencil(pass)?;
            if (uses_depth || uses_stencil) && texture.depth_texture.is_none() {
                return Err("Depth texture is required for depth or stencil testing".to_string());
            }

            let color = Some(wgpu::RenderPassColorAttachment {
                view: &texture.texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: pass
                        .clear_options
                        .color
                        .map_or(wgpu::LoadOp::Load, |color| {
                            wgpu::LoadOp::Clear(color.as_wgpu())
                        }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            });
            let depth = uses_depth.then(|| wgpu::Operations {
                load: pass
                    .clear_options
                    .depth
                    .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                store: StoreOp::Store,
            });
            let stencil = uses_stencil.then(|| wgpu::Operations {
                load: pass
                    .clear_options
                    .stencil
                    .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                store: StoreOp::Store,
            });
            let depth_stencil_attachment = texture.depth_texture.as_ref().and_then(|texture| {
                (depth.is_some() || stencil.is_some()).then_some(
                    wgpu::RenderPassDepthStencilAttachment {
                        view: &texture.view,
                        depth_ops: depth,
                        stencil_ops: stencil,
                    },
                )
            });

            let mut raw_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[color],
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let size = texture.texture.raw.size();
            encode_render_commands(&mut raw_pass, &pass.commands, size.width, size.height)?;
        }

        if !renderer.passes.is_empty() {
            self.ctx.queue.submit(Some(encoder.finish()));
            texture.texture.mark_written();
            self.current_stats.draw_calls += 1;
        }
        Ok(())
    }

    fn create_render_pipeline(
        &mut self,
        desc: RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, String> {
        log::debug!("Creating RenderPipeline (label={:?})", desc.label);

        let shader = self
            .ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: desc.label,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(desc.shader)),
            });

        let bind_group_layout =
            desc.bind_group_layout
                .iter()
                .map(|layout| {
                    let raw = self.ctx.device.create_bind_group_layout(
                        &wgpu::BindGroupLayoutDescriptor {
                            label: desc.label,
                            entries: &layout
                                .entries
                                .iter()
                                .map(|entry| {
                                    let visibility = wgpu_shader_visibility(
                                        entry.visible_vertex,
                                        entry.visible_fragment,
                                        entry.visible_compute,
                                    );
                                    let binding = entry.location;
                                    let ty = match entry.typ {
                                        BindType::Texture => wgpu::BindingType::Texture {
                                            multisampled: false,
                                            view_dimension: wgpu::TextureViewDimension::D2,
                                            sample_type: wgpu::TextureSampleType::Float {
                                                filterable: true,
                                            },
                                        },
                                        BindType::Sampler => wgpu::BindingType::Sampler(
                                            wgpu::SamplerBindingType::Filtering,
                                        ),
                                        BindType::Uniform => wgpu::BindingType::Buffer {
                                            ty: wgpu::BufferBindingType::Uniform,
                                            has_dynamic_offset: false,
                                            min_binding_size: None,
                                        },
                                    };
                                    wgpu::BindGroupLayoutEntry {
                                        binding,
                                        visibility,
                                        ty,
                                        count: None,
                                    }
                                })
                                .collect::<Vec<_>>(),
                        },
                    );

                    BindGroupLayoutRef {
                        id: resource_id(&mut self.next_resource_id),
                        raw: Arc::new(raw),
                    }
                })
                .collect::<ArrayVec<_, MAX_BIND_GROUPS_PER_PIPELINE>>();

        let mut compatible_formats = desc
            .compatible_textures
            .iter()
            .map(|format| format.as_wgpu())
            .collect::<ArrayVec<_, MAX_PIPELINE_COMPATIBLE_TEXTURES>>();
        if compatible_formats.is_empty() {
            compatible_formats.push(self.surface.config.format);
        }

        let blend = desc.blend_mode.map(|mode| mode.as_wgpu());
        let write_mask = desc.color_mask.as_wgpu();
        let targets = compatible_formats
            .iter()
            .map(|format| {
                Some(wgpu::ColorTargetState {
                    format: *format,
                    blend,
                    write_mask,
                })
            })
            .collect();

        let recipe = PipelineRecipe {
            label: desc.label.map(str::to_owned),
            shader,
            vertex_layout: desc.vertex_layout,
            primitive: desc.primitive,
            cull_mode: desc.cull_mode,
            depth: desc.depth_stencil,
            stencil: desc.stencil,
            targets,
            vs_entry: desc.vs_entry.map(str::to_owned),
            fs_entry: desc.fs_entry.map(str::to_owned),
        };
        let raw = self.build_render_pipeline(&recipe, &bind_group_layout);

        Ok(RenderPipeline {
            inner: Arc::new(PipelineInner {
                id: resource_id(&mut self.next_resource_id),
                raw,
                index_format: desc.index_format.as_wgpu(),
                uses_depth: recipe.depth.is_some(),
                uses_stencil: recipe.stencil.is_some(),
                bind_group_layout,
                recipe,
            }),
        })
    }

    fn create_stencil_variant(
        &mut self,
        base: &RenderPipeline,
        stencil: Stencil,
    ) -> Result<RenderPipeline, String> {
        let mut recipe = base.inner.recipe.clone();
        recipe.stencil = Some(stencil);
        let raw = self.build_render_pipeline(&recipe, &base.inner.bind_group_layout);

        Ok(RenderPipeline {
            inner: Arc::new(PipelineInner {
                id: resource_id(&mut self.next_resource_id),
                raw,
                index_format: base.inner.index_format,
                uses_depth: recipe.depth.is_some(),
                uses_stencil: true,
                bind_group_layout: base.inner.bind_group_layout.clone(),
                recipe,
            }),
        })
    }

    fn create_buffer(&mut self, desc: BufferDescriptor) -> Result<Buffer, String> {
        log::trace!(
            "Creating Buffer (label={:?}, usage={:?})",
            desc.label,
            desc.usage
        );
        let mut usage = desc.usage.as_wgpu();
        if desc.write {
            usage |= wgpu::BufferUsages::COPY_DST;
        }

        let (raw, size) = if desc.content.is_empty() {
            let size = 1024;
            let raw = self.ctx.device.create_buffer(&WBufferDescriptor {
                label: desc.label,
                size,
                usage,
                mapped_at_creation: false,
            });
            (raw, size as usize)
        } else {
            let raw = self.ctx.device.create_buffer_init(&BufferInitDescriptor {
                label: desc.label,
                contents: desc.content,
                usage,
            });
            (raw, desc.content.len())
        };

        let usage = desc.usage;

        Ok(Buffer {
            id: resource_id(&mut self.next_resource_id),
            inner: Arc::new(AtomicRefCell::new(InnerBuffer {
                size,
                raw: Arc::new(raw),
            })),
            usage,
            write: desc.write,
            inner_label: Arc::new(desc.label.map_or_else(|| "".to_string(), |l| l.to_string())),
        })
    }

    fn create_bind_group(&mut self, desc: BindGroupDescriptor) -> Result<BindGroup, String> {
        let mut next_resource_id = self.next_resource_id;
        let bind_group = self.prepare_bind_group(&mut next_resource_id, desc)?;
        self.next_resource_id = next_resource_id;
        Ok(bind_group)
    }

    fn write_buffer(&mut self, buffer: &Buffer, offset: u64, data: &[u8]) -> Result<(), String> {
        debug_assert!(buffer.write, "Cannot write data to a static buffer");

        // update inner buffer if the size is not enough
        if buffer.size() < data.len() {
            let required = offset as usize + data.len();
            let next_size = next_buffer_size(buffer.size(), required);

            log::debug!(
                "Updating Buffer '{}' size from {} to {}",
                buffer.inner_label,
                buffer.size(),
                next_size
            );
            let mut usage = buffer.usage.as_wgpu();
            usage |= wgpu::BufferUsages::COPY_DST;

            let raw = self.ctx.device.create_buffer(&WBufferDescriptor {
                label: buffer.inner_label.as_str().into(),
                size: next_size as _,
                usage,
                mapped_at_creation: false,
            });

            // copy current memory to the new one
            if offset > 0 {
                log::debug!(
                    "Copying Buffer '{}' memory until offset {}",
                    buffer.inner_label,
                    offset
                );
                let mut encoder =
                    self.ctx
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Buffer Copy Encoder"),
                        });

                encoder.copy_buffer_to_buffer(&buffer.inner.borrow().raw, 0, &raw, 0, offset);

                self.ctx.queue.submit(Some(encoder.finish()));
                self.current_stats.draw_calls += 1;
            }

            *buffer.inner.borrow_mut() = InnerBuffer {
                size: next_size,
                raw: Arc::new(raw),
            };
        }

        debug_assert!(
            buffer.size() >= offset as usize + data.len(),
            "Invalid buffer size '{}' expected '{}'",
            buffer.size(),
            offset as usize + data.len()
        );
        self.ctx
            .queue
            .write_buffer(&buffer.inner.borrow().raw, offset as _, data);
        Ok(())
    }

    fn create_sampler(&mut self, desc: SamplerDescriptor) -> Result<Sampler, String> {
        log::trace!("Creating Sampler (label={:?})", desc.label);
        let raw = self.ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: desc.label,
            address_mode_u: desc.wrap_x.as_wgpu(),
            address_mode_v: desc.wrap_y.as_wgpu(),
            address_mode_w: desc.wrap_z.as_wgpu(),
            mag_filter: desc.mag_filter.as_wgpu(),
            min_filter: desc.min_filter.as_wgpu(),
            mipmap_filter: desc
                .mipmap_filter
                .map_or(Default::default(), |tf| tf.as_wgpu_mipmap()),
            ..Default::default()
        });
        Ok(Sampler {
            id: resource_id(&mut self.next_resource_id),
            raw: Arc::new(raw),
            wrap_x: desc.wrap_x,
            wrap_y: desc.wrap_y,
            wrap_z: desc.wrap_z,
            mag_filter: desc.mag_filter,
            min_filter: desc.min_filter,
            // mipmap_filter: desc.mipmap_filter,
        })
    }

    fn create_texture(
        &mut self,
        desc: TextureDescriptor,
        data: Option<TextureData>,
    ) -> Result<Texture, String> {
        log::trace!("Creating Texture (label={:?})", desc.label);
        let id = resource_id(&mut self.next_resource_id);
        create_texture(
            id,
            &self.ctx.device,
            &self.ctx.queue,
            self.ctx.supports_view_formats,
            desc,
            data,
        )
    }

    fn write_texture(
        &mut self,
        texture: &Texture,
        offset: UVec2,
        size: UVec2,
        data: &[u8],
    ) -> Result<(), String> {
        debug_assert!(
            texture.write,
            "Cannot update an immutable texture '{:?}'",
            texture.id()
        );
        let channels = data.len() as u32 / (size.element_product());
        let mut copy = texture.raw.as_image_copy();
        copy.origin = Origin3d {
            x: offset.x,
            y: offset.y,
            z: 0,
        };
        self.ctx.queue.write_texture(
            copy,
            data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.x * channels),
                rows_per_image: None,
            },
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
        );
        texture.mark_written();
        Ok(())
    }

    fn create_render_texture(
        &mut self,
        desc: RenderTextureDescriptor,
    ) -> Result<RenderTexture, String> {
        let mut next_resource_id = self.next_resource_id;
        let texture = self.prepare_render_texture(&mut next_resource_id, desc)?;
        self.next_resource_id = next_resource_id;
        Ok(texture)
    }

    fn limits(&self) -> Limits {
        let raw_limits = self.ctx.device.limits();
        let surface_formats = self
            .surface
            .capabilities
            .formats
            .iter()
            .filter_map(|tx| TextureFormat::from_wgpu(*tx))
            .collect();

        Limits {
            max_texture_size_2d: raw_limits.max_texture_dimension_2d,
            max_texture_size_3d: raw_limits.max_texture_dimension_3d,
            surface_formats,
        }
    }

    fn stats(&self) -> GpuStats {
        self.last_frame_stats
    }
}

#[inline(always)]
fn resource_id<T: From<u64>>(count: &mut u64) -> T {
    let id = *count;
    *count += 1;
    T::from(id)
}

impl GfxBackend {
    fn build_render_pipeline(
        &self,
        recipe: &PipelineRecipe,
        bind_group_layouts: &ArrayVec<BindGroupLayoutRef, MAX_BIND_GROUPS_PER_PIPELINE>,
    ) -> wgpu::RenderPipeline {
        let layout_refs = bind_group_layouts
            .iter()
            .map(|layout| Some(layout.raw.as_ref()))
            .collect::<Vec<_>>();
        let pipeline_layout =
            self.ctx
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: recipe.label.as_deref(),
                    bind_group_layouts: &layout_refs,
                    immediate_size: 0,
                });

        let (attributes, mut buffers): (Vec<_>, Vec<_>) = recipe
            .vertex_layout
            .iter()
            .map(|layout| {
                let mut offset = 0;
                let attributes = layout
                    .attributes
                    .iter()
                    .map(|attribute| {
                        let attribute = wgpu::VertexAttribute {
                            format: attribute.format.as_wgpu(),
                            offset,
                            shader_location: attribute.location,
                        };
                        offset += attribute.format.size();
                        attribute
                    })
                    .collect::<Vec<_>>();
                let buffer = wgpu::VertexBufferLayout {
                    array_stride: offset,
                    step_mode: layout.step_mode.as_wgpu(),
                    attributes: &[],
                };
                (attributes, buffer)
            })
            .unzip();
        for (index, buffer) in buffers.iter_mut().enumerate() {
            buffer.attributes = &attributes[index];
        }
        let buffers = buffers.into_iter().map(Some).collect::<Vec<_>>();

        self.ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: recipe.label.as_deref(),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &recipe.shader,
                    entry_point: recipe.vs_entry.as_deref().or(Some("vs_main")),
                    compilation_options: Default::default(),
                    buffers: &buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &recipe.shader,
                    entry_point: recipe.fs_entry.as_deref().or(Some("fs_main")),
                    compilation_options: Default::default(),
                    targets: recipe.targets.as_slice(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: recipe.primitive.as_wgpu(),
                    cull_mode: recipe.cull_mode.map(|mode| mode.as_wgpu()),
                    ..Default::default()
                },
                depth_stencil: wgpu_depth_stencil(recipe.depth, recipe.stencil),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
    }

    pub(crate) async fn init(
        source: SurfaceSource,
        vsync: bool,
        win_size: UVec2,
        pixelated: bool,
    ) -> Result<Self, String> {
        let is_zero = win_size.x == 0 || win_size.y == 0;
        if is_zero {
            return Err("Cannot initialize a surface with a zero size".to_string());
        }

        let backend_override = Backends::from_env().filter(|backends| !backends.is_empty());

        #[cfg(target_arch = "wasm32")]
        {
            init_wasm_gfx(source, vsync, win_size, pixelated, backend_override).await
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            init_native_gfx(source, vsync, win_size, pixelated, backend_override).await
        }
    }

    #[cfg(native_windowed)]
    pub(crate) fn update_surface(
        &mut self,
        source: SurfaceSource,
        win_size: UVec2,
    ) -> Result<(), String> {
        let size = if win_size.x == 0 || win_size.y == 0 {
            UVec2::new(self.surface.config.width, self.surface.config.height)
        } else {
            win_size
        };
        if size.x == 0 || size.y == 0 {
            return Err("Cannot replace a surface without a nonzero size".to_string());
        }

        let owner = SurfaceOwner::new(source, &self.ctx.instance)?;
        let candidate = SurfaceCandidate::replacement(&self.ctx, owner, size, &self.surface)?;
        let mut next_resource_id = self.next_resource_id;
        let depth_texture = create_surface_depth(
            resource_id(&mut next_resource_id),
            &self.ctx,
            self.depth_format,
            size,
        )?;
        let offscreen = self
            .offscreen
            .as_ref()
            .ok_or_else(|| "Invalid Offscreen surface".to_string())?
            .prepare_resize(self, size, &mut next_resource_id)?;

        let surface = candidate.configure(&self.ctx.device, depth_texture);

        self.surface = surface;
        if let Some(resized) = offscreen {
            self.offscreen.as_mut().unwrap().commit_resize(resized);
        }
        self.next_resource_id = next_resource_id;
        self.frame = None;
        Ok(())
    }

    pub(crate) fn prepare_bind_group(
        &self,
        next_resource_id: &mut u64,
        desc: BindGroupDescriptor,
    ) -> Result<BindGroup, String> {
        log::trace!("Creating BindGroup (label={:?})", desc.label);
        let buffers: ArrayVec<_, MAX_BINDING_ENTRIES> = desc
            .entry
            .iter()
            .map(|entry| match entry {
                BindGroupEntry::Uniform { buffer, .. } => Some(buffer.inner.borrow().raw.clone()),
                _ => None,
            })
            .collect();
        let entries: ArrayVec<_, MAX_BINDING_ENTRIES> = desc
            .entry
            .iter()
            .enumerate()
            .map(|(idx, entry)| match entry {
                BindGroupEntry::Texture { location, texture } => wgpu::BindGroupEntry {
                    binding: *location,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                BindGroupEntry::Uniform { location, .. } => wgpu::BindGroupEntry {
                    binding: *location,
                    resource: buffers[idx].as_ref().unwrap().as_entire_binding(),
                },
                BindGroupEntry::Sampler { location, sampler } => wgpu::BindGroupEntry {
                    binding: *location,
                    resource: wgpu::BindingResource::Sampler(&sampler.raw),
                },
            })
            .collect();
        let layout = desc
            .layout
            .ok_or("Cannot create binding group with a missing layout.")?;
        let raw = self
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: desc.label,
                layout: &layout.raw,
                entries: &entries,
            });

        Ok(BindGroup {
            id: resource_id(next_resource_id),
            raw: Arc::new(raw),
        })
    }

    pub(crate) fn prepare_render_texture(
        &self,
        next_resource_id: &mut u64,
        desc: RenderTextureDescriptor,
    ) -> Result<RenderTexture, String> {
        log::trace!("Creating RenderTexture (label={:?})", desc.label);
        let format = match desc.format {
            Some(format) => format,
            None => TextureFormat::from_wgpu(self.surface.config.format)
                .ok_or_else(|| "Unsupported surface texture format".to_string())?,
        };
        let color_label = format!("RenderTexture (label={:?}) inner color texture", desc.label);
        let texture = create_texture(
            resource_id(next_resource_id),
            &self.ctx.device,
            &self.ctx.queue,
            self.ctx.supports_view_formats,
            TextureDescriptor {
                label: Some(&color_label),
                format,
                write: true,
            },
            Some(TextureData {
                bytes: &[],
                width: desc.width,
                height: desc.height,
            }),
        )?;
        let depth_texture = if desc.depth {
            let depth_label = format!("RenderTexture (label={:?}) inner depth texture", desc.label);
            Some(create_texture(
                resource_id(next_resource_id),
                &self.ctx.device,
                &self.ctx.queue,
                self.ctx.supports_view_formats,
                TextureDescriptor {
                    label: Some(&depth_label),
                    format: SURFACE_DEFAULT_DEPTH_FORMAT,
                    write: true,
                },
                Some(TextureData {
                    bytes: &[],
                    width: desc.width,
                    height: desc.height,
                }),
            )?)
        } else {
            None
        };

        Ok(RenderTexture {
            id: resource_id(next_resource_id),
            texture,
            depth_texture,
        })
    }

    async fn new(
        source: SurfaceSource,
        vsync: bool,
        win_size: UVec2,
        pixelated: bool,
        backends: Backends,
    ) -> Result<Self, String> {
        let depth_format = SURFACE_DEFAULT_DEPTH_FORMAT; // make it configurable?
        let mut next_resource_id = 0;

        #[cfg(not(target_arch = "wasm32"))]
        let mut descriptor =
            InstanceDescriptor::new_with_display_handle(Box::new(source.display.clone()));
        #[cfg(target_arch = "wasm32")]
        let mut descriptor = InstanceDescriptor::new_with_display_handle(Box::new(WebDisplay));
        descriptor.backends = backends;
        descriptor.flags = InstanceFlags::from_env_or_default();
        descriptor.backend_options = BackendOptions {
            gl: GlBackendOptions::from_env_or_default(),
            dx12: Dx12BackendOptions {
                shader_compiler: wgpu::Dx12Compiler::StaticDxc,
                ..Dx12BackendOptions::from_env_or_default()
            },
            ..BackendOptions::from_env_or_default()
        };

        // this will automatically fallback to webgl if webgpu is not supported
        let instance = new_instance_with_webgpu_detection(descriptor).await;

        let (ctx, surface) = {
            let owner = SurfaceOwner::new(source, &instance)?;
            let ctx = Context::new(instance, owner.raw()).await?;
            let candidate = SurfaceCandidate::initial(&ctx, owner, win_size, vsync)?;
            let depth_texture = create_surface_depth(
                resource_id(&mut next_resource_id),
                &ctx,
                depth_format,
                win_size,
            )?;
            let surface = candidate.configure(&ctx.device, depth_texture);
            (ctx, surface)
        };

        let mut bck = Self {
            next_resource_id,
            ctx,
            #[cfg(native_windowed)]
            depth_format,
            surface,
            frame: None,
            offscreen: None,
            last_frame_stats: GpuStats::default(),
            current_stats: GpuStats::default(),
        };

        let offscreen = OffscreenSurfaceData::new(&mut bck, pixelated)?;
        bck.offscreen = Some(offscreen);

        Ok(bck)
    }

    fn push_frame(&mut self) -> Result<(), String> {
        let (frame, reconfigure_after_frame) = match self.surface.frame() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                let config = &self.surface.config;
                let size = UVec2::new(config.width, config.height);
                self.try_resize(size.x, size.y)?;
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.recreate(&self.ctx).map_err(|e| {
                    format!(
                        "Cannot recover lost WGPU surface while preserving the existing adapter, device, and color format: {e}"
                    )
                })?;
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("WGPU surface texture acquisition failed validation".to_string());
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Frame Encode"),
            });
        self.frame = Some(DrawFrame {
            frame,
            view,
            encoder,
            dirty: false,
            reconfigure_after_frame,
        });

        Ok(())
    }

    pub(crate) fn render_to_frame(
        &mut self,
        frame: &mut DrawFrame,
        renderer: &Renderer,
    ) -> Result<(), String> {
        for pass in &renderer.passes {
            let (uses_depth, uses_stencil) = pass_uses_depth_stencil(pass)?;
            let color = Some(wgpu::RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: pass
                        .clear_options
                        .color
                        .map_or(wgpu::LoadOp::Load, |color| {
                            wgpu::LoadOp::Clear(color.as_wgpu())
                        }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            });
            let depth = uses_depth.then(|| wgpu::Operations {
                load: pass
                    .clear_options
                    .depth
                    .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                store: StoreOp::Store,
            });
            let stencil = uses_stencil.then(|| wgpu::Operations {
                load: pass
                    .clear_options
                    .stencil
                    .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                store: StoreOp::Store,
            });

            let mut raw_pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[color],
                    depth_stencil_attachment: (depth.is_some() || stencil.is_some()).then_some(
                        wgpu::RenderPassDepthStencilAttachment {
                            view: &self.surface.depth_texture.view,
                            depth_ops: depth,
                            stencil_ops: stencil,
                        },
                    ),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
            encode_render_commands(
                &mut raw_pass,
                &pass.commands,
                self.surface.config.width,
                self.surface.config.height,
            )?;
        }
        Ok(())
    }

    fn present_to_screen(&mut self) {
        let Some(mut df) = self.frame.take() else {
            return;
        };
        let reconfigure_after_frame = df.reconfigure_after_frame;

        if df.dirty {
            // TODO change this: "take" is ugly as hell
            let offscreen = self.offscreen.take().unwrap();
            offscreen.present(self, &mut df).unwrap();
            self.offscreen = Some(offscreen);

            let DrawFrame {
                frame,
                view,
                encoder,
                ..
            } = df;
            self.ctx.queue.submit(Some(encoder.finish()));
            self.current_stats.draw_calls += 1;
            drop(view);
            self.ctx.queue.present(frame);
        } else {
            drop(df);
        }

        if reconfigure_after_frame {
            self.surface.reconfigure(&self.ctx.device);
        }
    }

    #[inline]
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if let Err(err) = self.try_resize(width, height) {
            log::error!("Error resizing Gfx backend: {err}");
        }
    }

    fn try_resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Ok(());
        }

        let size = UVec2::new(width, height);
        let mut next_resource_id = self.next_resource_id;
        let resized = self
            .offscreen
            .as_ref()
            .ok_or_else(|| "Invalid Offscreen surface".to_string())?
            .prepare_resize(self, size, &mut next_resource_id)?;

        self.surface.configure_size(&self.ctx.device, size);
        if let Some(resized) = resized {
            self.offscreen.as_mut().unwrap().commit_resize(resized);
        }
        self.next_resource_id = next_resource_id;
        Ok(())
    }
}

fn create_surface_depth(
    id: TextureId,
    ctx: &Context,
    depth_format: TextureFormat,
    size: UVec2,
) -> Result<Texture, String> {
    create_texture(
        id,
        &ctx.device,
        &ctx.queue,
        ctx.supports_view_formats,
        TextureDescriptor {
            label: Some("Depth Texture for Surface"),
            format: depth_format,
            write: true,
        },
        Some(TextureData {
            bytes: &[],
            width: size.x,
            height: size.y,
        }),
    )
}

fn create_texture(
    id: TextureId,
    device: &wgpu::Device,
    queue: &Queue,
    supports_view_formats: bool,
    desc: TextureDescriptor,
    data: Option<TextureData>,
) -> Result<Texture, String> {
    let size = data.map_or(wgpu::Extent3d::default(), |d| wgpu::Extent3d {
        width: d.width,
        height: d.height,
        depth_or_array_layers: 1,
    });

    let is_depth_texture = matches!(desc.format, SURFACE_DEFAULT_DEPTH_FORMAT);
    let mut usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    if is_depth_texture || desc.write {
        usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    }

    let view_formats = desc.format.view_formats();

    let raw = device.create_texture(&wgpu::TextureDescriptor {
        label: desc.label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: desc.format.as_wgpu(),
        usage,
        view_formats: if supports_view_formats {
            &view_formats
        } else {
            &[]
        },
    });

    if !is_depth_texture && let Some(d) = data {
        // TODO, get the bytes_per_row/channles from the TextureFormat instead?

        let total = d.width * d.height;
        debug_assert!(total != 0, "Depth texture width or height cannot be zero.");
        let channels = d.bytes.len() as u32 / total;
        if !d.bytes.is_empty() {
            queue.write_texture(
                raw.as_image_copy(),
                d.bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(d.width * channels),
                    rows_per_image: Some(d.height),
                },
                size,
            );
        }
    }

    let view = raw.create_view(&wgpu::TextureViewDescriptor {
        format: Some(desc.format.as_wgpu()),
        ..Default::default()
    });

    Ok(Texture {
        id,
        raw: Arc::new(raw),
        view: Arc::new(view),
        size: vec2(size.width as _, size.height as _),
        write: desc.write,
        format: desc.format,
        revision: Arc::new(AtomicU32::new(0)),
    })
}

impl Color {
    pub(crate) fn as_wgpu(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }
}

fn next_buffer_size(current: usize, required: usize) -> usize {
    current.max(required).next_power_of_two()
}
