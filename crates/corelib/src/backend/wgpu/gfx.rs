#![allow(clippy::arc_with_non_send_sync)]

use crate::{
    backend::{
        traits::{GfxBackendImpl, SurfaceSource},
        wgpu::{
            context::Context,
            frame::DrawFrame,
            offscreen::OffscreenSurfaceData,
            surface::{Surface, SurfaceCandidate, SurfaceOwner},
            utils::{wgpu_depth_stencil, wgpu_shader_visibility},
        },
    },
    gfx::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutRef, BindType, Buffer,
        BufferDescriptor, BufferUsage, Color, GpuStats, InnerBuffer, Limits, MAX_BINDING_ENTRIES,
        RenderPipeline, RenderPipelineDescriptor, RenderTexture, RenderTextureDescriptor, Renderer,
        Sampler, SamplerDescriptor, Texture, TextureData, TextureDescriptor, TextureFormat,
        TextureId,
        consts::{MAX_PIPELINE_COMPATIBLE_TEXTURES, SURFACE_DEFAULT_DEPTH_FORMAT},
    },
    math::{UVec2, vec2},
};
use arrayvec::ArrayVec;
use atomic_refcell::AtomicRefCell;
#[cfg(target_arch = "wasm32")]
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};
use std::{borrow::Cow, sync::Arc};
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

    #[cfg(all(not(target_arch = "wasm32"), not(feature = "headless")))]
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
    async fn init(
        source: SurfaceSource,
        vsync: bool,
        win_size: UVec2,
        pixelated: bool,
    ) -> Result<Self, String>
    where
        Self: Sized,
    {
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

    #[cfg(all(not(target_arch = "wasm32"), not(feature = "headless")))]
    fn update_surface(&mut self, source: SurfaceSource, win_size: UVec2) -> Result<(), String>
    where
        Self: Sized,
    {
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

        let can_render = texture.size.x > 0.0 && texture.size.y > 0.0;
        if !can_render {
            // skip rendering when minized on window
            return Ok(());
        }

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RenderTexture Encoder"),
            });

        renderer
            .passes
            .iter()
            .try_for_each(|rp| -> Result<(), String> {
                let (uses_depth, uses_stencil) = rp
                    .pipeline
                    .map_or((false, false), |pip| (pip.uses_depth, pip.uses_stencil));

                let needs_depth_stencil = uses_depth || uses_stencil;
                if needs_depth_stencil && texture.depth_texture.is_none() {
                    return Err(
                        "Depth texture is required for depth or stencil testing".to_string()
                    );
                }

                let color = Some(rp.clear_options.color.map_or_else(
                    || wgpu::RenderPassColorAttachment {
                        view: &texture.texture.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    },
                    |_color| wgpu::RenderPassColorAttachment {
                        view: &texture.texture.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: rp.clear_options.color.map_or(wgpu::LoadOp::Load, |color| {
                                wgpu::LoadOp::Clear(color.as_wgpu())
                            }),
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    },
                ));

                let depth = if uses_depth {
                    Some(wgpu::Operations {
                        load: rp
                            .clear_options
                            .depth
                            .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                        store: StoreOp::Store,
                    })
                } else {
                    None
                };

                let stencil = if uses_stencil {
                    Some(wgpu::Operations {
                        load: rp
                            .clear_options
                            .stencil
                            .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                        store: StoreOp::Store,
                    })
                } else {
                    None
                };

                let depth_stencil_attachment = texture.depth_texture.as_ref().and_then(|dt| {
                    if depth.is_some() || stencil.is_some() {
                        Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &dt.view,
                            depth_ops: depth,
                            stencil_ops: stencil,
                        })
                    } else {
                        None
                    }
                });

                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[color],
                    depth_stencil_attachment,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                if let Some(pip) = rp.pipeline {
                    rpass.set_pipeline(&pip.raw);

                    if let Some([x, y, width, height]) = rp.scissors {
                        rpass.set_scissor_rect(x, y, width, height);
                    }

                    let mut vertex_buffers_slot = 0;
                    let mut indexed = false;
                    rp.buffers.iter().for_each(|buff| {
                        // debug_assert!(!buff.1.is_empty(), "Buffer offsets cannot be empty");
                        if buff.1.is_empty() {
                            log::warn!(
                                "Buffer '{} - ({:?})' offset is empty. Skipping...",
                                buff.0.inner_label,
                                buff.0.id
                            );
                            return;
                        }

                        match buff.0.usage {
                            BufferUsage::Vertex => {
                                rpass.set_vertex_buffer(
                                    vertex_buffers_slot,
                                    buff.0.inner.borrow().raw.slice(buff.1.clone()),
                                );
                                vertex_buffers_slot += 1;
                            }
                            BufferUsage::Index => {
                                debug_assert!(!indexed, "Cannot bind more than one Index buffer");
                                indexed = true;
                                rpass.set_index_buffer(
                                    buff.0.inner.borrow().raw.slice(buff.1.clone()),
                                    pip.index_format,
                                )
                            }
                            BufferUsage::Uniform => {}
                        }
                    });

                    rp.bind_groups.iter().enumerate().for_each(|(i, bg)| {
                        rpass.set_bind_group(i as _, &*bg.raw, &[]);
                    });

                    if let Some(sr) = rp.stencil_ref {
                        rpass.set_stencil_reference(sr as _);
                    }

                    rp.vertices.iter().for_each(|vertices| {
                        if !vertices.range.is_empty() {
                            let instances = 0..vertices.instances.unwrap_or(1);
                            if indexed {
                                rpass.draw_indexed(vertices.range.clone(), 0, instances);
                            } else {
                                rpass.draw(vertices.range.clone(), instances);
                            }
                        }
                    });
                }

                Ok(())
            })?;

        if !renderer.passes.is_empty() {
            self.ctx.queue.submit(Some(encoder.finish()));
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

        let mut bind_group_layouts = desc
            .bind_group_layout
            .iter()
            .map(|bgl| {
                self.ctx
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: desc.label,
                        entries: &bgl
                            .entries
                            .iter()
                            .map(|entry| {
                                let visibility = wgpu_shader_visibility(
                                    entry.visible_vertex,
                                    entry.visible_fragment,
                                    entry.visible_compute,
                                );
                                let binding = entry.location;
                                match entry.typ {
                                    BindType::Texture => wgpu::BindGroupLayoutEntry {
                                        binding,
                                        visibility,
                                        ty: wgpu::BindingType::Texture {
                                            multisampled: false,
                                            view_dimension: wgpu::TextureViewDimension::D2,
                                            sample_type: wgpu::TextureSampleType::Float {
                                                filterable: true,
                                            },
                                        },
                                        count: None,
                                    },
                                    BindType::Sampler => wgpu::BindGroupLayoutEntry {
                                        binding,
                                        visibility,
                                        ty: wgpu::BindingType::Sampler(
                                            wgpu::SamplerBindingType::Filtering,
                                        ),
                                        count: None,
                                    },
                                    BindType::Uniform => wgpu::BindGroupLayoutEntry {
                                        binding,
                                        visibility,
                                        ty: wgpu::BindingType::Buffer {
                                            ty: wgpu::BufferBindingType::Uniform,
                                            has_dynamic_offset: false,
                                            min_binding_size: None,
                                        },
                                        count: None,
                                    },
                                }
                            })
                            .collect::<Vec<_>>(),
                    })
            })
            .collect::<Vec<_>>();

        let layout_refs = bind_group_layouts.iter().map(Some).collect::<Vec<_>>();
        let pipeline_layout =
            self.ctx
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: desc.label,
                    bind_group_layouts: &layout_refs,
                    immediate_size: 0,
                });

        let (attrs, mut buffers): (Vec<_>, Vec<_>) = desc
            .vertex_layout
            .iter()
            .map(|vl| {
                let mut offset = 0;
                let attrs = vl
                    .attributes
                    .iter()
                    .map(|attr| {
                        let a = wgpu::VertexAttribute {
                            format: attr.format.as_wgpu(),
                            offset,
                            shader_location: attr.location as _,
                        };
                        offset += a.format.size();
                        a
                    })
                    .collect::<Vec<_>>();

                let layout = wgpu::VertexBufferLayout {
                    array_stride: offset,
                    step_mode: vl.step_mode.as_wgpu(),
                    attributes: &[],
                };

                (attrs, layout)
            })
            .unzip();

        buffers
            .iter_mut()
            .enumerate()
            .for_each(|(i, buff)| buff.attributes = &attrs[i]);
        let buffers = buffers.into_iter().map(Some).collect::<Vec<_>>();

        let mut compatible_formats = desc
            .compatible_textures
            .iter()
            .map(|tf| tf.as_wgpu())
            .collect::<ArrayVec<_, MAX_PIPELINE_COMPATIBLE_TEXTURES>>();

        if compatible_formats.is_empty() {
            compatible_formats.push(self.surface.config.format);
        }

        let blend = desc.blend_mode.map(|bm| bm.as_wgpu());
        let write_mask = desc.color_mask.as_wgpu();
        let fragment_targets = compatible_formats
            .iter()
            .map(|format| {
                let swapchain_color_target: wgpu::ColorTargetState = (*format).into();
                Some(wgpu::ColorTargetState {
                    blend,
                    write_mask,
                    ..swapchain_color_target
                })
            })
            .collect::<ArrayVec<_, MAX_PIPELINE_COMPATIBLE_TEXTURES>>();

        let raw = self
            .ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: desc.label,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: desc.vs_entry.or(Some("vs_main")),
                    compilation_options: Default::default(),
                    buffers: &buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: desc.fs_entry.or(Some("fs_main")),
                    compilation_options: Default::default(),
                    targets: fragment_targets.as_slice(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: desc.primitive.as_wgpu(),
                    cull_mode: desc.cull_mode.map(|cm| cm.as_wgpu()),
                    ..Default::default()
                },
                depth_stencil: wgpu_depth_stencil(desc.depth_stencil, desc.stencil),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        let index_format = desc.index_format.as_wgpu();
        let mut bind_group_layout = ArrayVec::new();
        bind_group_layouts.reverse();
        while let Some(bgl) = bind_group_layouts.pop() {
            bind_group_layout.push(BindGroupLayoutRef {
                id: resource_id(&mut self.next_resource_id),
                raw: Arc::new(bgl),
            });
        }
        Ok(RenderPipeline {
            id: resource_id(&mut self.next_resource_id),
            raw: Arc::new(raw),
            index_format,
            uses_depth: desc.depth_stencil.is_some(),
            uses_stencil: desc.stencil.is_some(),
            bind_group_layout,
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
            #[cfg(all(not(target_arch = "wasm32"), not(feature = "headless")))]
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
        renderer
            .passes
            .iter()
            .try_for_each(|rp| -> Result<(), String> {
                let (uses_depth, uses_stencil) = rp
                    .pipeline
                    .map_or((false, false), |pip| (pip.uses_depth, pip.uses_stencil));

                let color = Some(rp.clear_options.color.map_or_else(
                    || wgpu::RenderPassColorAttachment {
                        view: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    },
                    |_color| wgpu::RenderPassColorAttachment {
                        view: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: rp.clear_options.color.map_or(wgpu::LoadOp::Load, |color| {
                                wgpu::LoadOp::Clear(color.as_wgpu())
                            }),
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    },
                ));

                let depth = if uses_depth {
                    Some(wgpu::Operations {
                        load: rp
                            .clear_options
                            .depth
                            .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                        store: StoreOp::Store,
                    })
                } else {
                    None
                };

                let stencil = if uses_stencil {
                    Some(wgpu::Operations {
                        load: rp
                            .clear_options
                            .stencil
                            .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                        store: StoreOp::Store,
                    })
                } else {
                    None
                };

                let encoder = &mut frame.encoder;
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[color],
                    depth_stencil_attachment: if depth.is_some() || stencil.is_some() {
                        Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &self.surface.depth_texture.view,
                            depth_ops: depth,
                            stencil_ops: stencil,
                        })
                    } else {
                        None
                    },
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                if let Some(pip) = rp.pipeline {
                    rpass.set_pipeline(&pip.raw);

                    let mut vertex_buffers_slot = 0;
                    let mut indexed = false;
                    rp.buffers.iter().for_each(|buff| match buff.0.usage {
                        BufferUsage::Vertex => {
                            rpass.set_vertex_buffer(
                                vertex_buffers_slot,
                                buff.0.inner.borrow().raw.slice(buff.1.clone()),
                            );
                            vertex_buffers_slot += 1;
                        }
                        BufferUsage::Index => {
                            debug_assert!(!indexed, "Cannot bind more than one Index buffer");
                            indexed = true;
                            rpass.set_index_buffer(
                                buff.0.inner.borrow().raw.slice(buff.1.clone()),
                                pip.index_format,
                            )
                        }
                        BufferUsage::Uniform => {}
                    });

                    rp.bind_groups.iter().enumerate().for_each(|(i, bg)| {
                        rpass.set_bind_group(i as _, &*bg.raw, &[]);
                    });

                    if let Some(sr) = rp.stencil_ref {
                        rpass.set_stencil_reference(sr as _);
                    }

                    rp.vertices.iter().for_each(|vertices| {
                        if !vertices.range.is_empty() {
                            let instances = 0..vertices.instances.unwrap_or(1);
                            if indexed {
                                rpass.draw_indexed(vertices.range.clone(), 0, instances);
                            } else {
                                rpass.draw(vertices.range.clone(), instances);
                            }
                        }
                    });
                }

                Ok(())
            })?;

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
