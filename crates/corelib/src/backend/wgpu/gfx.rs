#![allow(clippy::arc_with_non_send_sync)]

use crate::backend::traits::GfxBackendImpl;
use crate::backend::wgpu::context::Context;
use crate::backend::wgpu::frame::{self, DrawFrame};
use crate::backend::wgpu::offscreen::OffscreenSurfaceData;
use crate::backend::wgpu::surface::Surface;
use crate::backend::wgpu::utils::{wgpu_depth_stencil, wgpu_shader_visibility};
use crate::gfx::consts::{MAX_PIPELINE_COMPATIBLE_TEXTURES, SURFACE_DEFAULT_DEPTH_FORMAT};
use crate::gfx::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutRef, BindType, Buffer,
    BufferDescriptor, BufferUsage, Color, GpuStats, InnerBuffer, Limits, RenderPipeline,
    RenderPipelineDescriptor, RenderTexture, RenderTextureDescriptor, Renderer, Texture,
    TextureData, TextureDescriptor, TextureFormat, TextureId,
};
use crate::gfx::{MAX_BINDING_ENTRIES, Sampler, SamplerDescriptor};
use crate::math::{UVec2, vec2};
use arrayvec::ArrayVec;
use atomic_refcell::AtomicRefCell;
use glam::uvec2;
use std::borrow::Cow;
use std::sync::Arc;
use utils::helpers::next_pot2;
use wgpu::rwh::HasWindowHandle;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BackendOptions, Backends, BufferDescriptor as WBufferDescriptor, Extent3d, GlBackendOptions,
    Gles3MinorVersion, Instance, InstanceDescriptor, InstanceFlags, Origin3d, Queue, StoreOp,
    Surface as RawSurface, TexelCopyBufferLayout, TextureDimension,
    TextureFormat as WTextureFormat,
};
use winit::raw_window_handle::HasDisplayHandle;

pub(crate) struct GfxBackend {
    pub(crate) surface: Surface, // Eventually we could have a HashMap<WindowId, Surface> if we want multiple window

    next_resource_id: u64,
    ctx: Context,

    #[cfg_attr(target_arch = "wasm32", allow(unused))]
    depth_format: TextureFormat,
    frame: Option<DrawFrame>,

    // used as intermediate for surface and pipeline texture formats
    offscreen: Option<OffscreenSurfaceData>,

    last_size: UVec2,

    last_frame_stats: GpuStats,
    current_stats: GpuStats,
}

// This is a hack for wasm32 browsers where there is no threads
#[cfg(target_arch = "wasm32")]
unsafe impl Send for GfxBackend {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for GfxBackend {}

impl GfxBackendImpl for GfxBackend {
    async fn init<W>(
        window: &W,
        vsync: bool,
        win_size: UVec2,
        pixelated: bool,
    ) -> Result<Self, String>
    where
        Self: Sized,
        W: HasDisplayHandle + HasWindowHandle,
    {
        Self::new(window, vsync, win_size, pixelated).await
    }

    async fn update_surface<W>(
        &mut self,
        window: &W,
        vsync: bool,
        win_size: UVec2,
    ) -> Result<(), String>
    where
        Self: Sized,
        W: HasDisplayHandle + HasWindowHandle,
    {
        let id = resource_id(&mut self.next_resource_id);
        let surface = Surface::create_raw_surface(window, &self.ctx.instance)?;
        let surface = init_surface_from_raw(
            id,
            &mut self.ctx,
            surface,
            self.depth_format,
            win_size,
            vsync,
        )
        .await?;
        self.surface = surface;
        Ok(())
    }

    fn prepare_frame(&mut self) {
        let can_render = self.surface.config.width > 0 && self.surface.config.height > 0;
        if !can_render {
            // on win_os minized windows can report 0 size, skip rendering
            return;
        }

        if let Err(e) = self.push_frame() {
            log::error!("Error creating frame: {e}");
        }
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

        if !renderer.passes.is_empty() {
            if let Some(frame) = &mut self.frame {
                frame.dirty = true;
            }
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

                let color = Some(rp.clear_options.color.map_or_else(
                    || wgpu::RenderPassColorAttachment {
                        view: &texture.texture.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: StoreOp::Store,
                        },
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
                });

                if let Some(pip) = rp.pipeline {
                    rpass.set_pipeline(&pip.raw);

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

        let pipeline_layout =
            self.ctx
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: desc.label,
                    bind_group_layouts: &bind_group_layouts.iter().collect::<Vec<&_>>(),
                    push_constant_ranges: &[],
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

        // compatible formats by default for pipelines
        let mut compatible_formats: ArrayVec<WTextureFormat, MAX_PIPELINE_COMPATIBLE_TEXTURES> =
            ArrayVec::new();
        desc.compatible_textures.iter().for_each(|tf| {
            let wgpu_tf = tf.as_wgpu();
            if compatible_formats.contains(&wgpu_tf) {
                return;
            }

            compatible_formats.push(wgpu_tf);
        });

        if compatible_formats.is_empty() {
            compatible_formats.push(TextureFormat::Rgba8UNormSrgb.as_wgpu());
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
                multiview: None,
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
        // NOTE: borrow checker hack to reference Arc<Buffer> later
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
                BindGroupEntry::Uniform {
                    location,
                    buffer: _,
                } => wgpu::BindGroupEntry {
                    binding: *location,
                    // NOTE: hacky as hell... this is made to please the borrow checker,
                    // we need to reference a buffer who lives outside of this loop
                    resource: buffers[idx].as_ref().unwrap().as_entire_binding(),
                },
                BindGroupEntry::Sampler { location, sampler } => wgpu::BindGroupEntry {
                    binding: *location,
                    resource: wgpu::BindingResource::Sampler(&sampler.raw),
                },
            })
            .collect();

        let raw = self
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: desc.label,
                layout: &desc
                    .layout
                    .ok_or("Cannot create binding group with a missing layout.")?
                    .raw,
                entries: &entries,
            });

        Ok(BindGroup {
            id: resource_id(&mut self.next_resource_id),
            raw: Arc::new(raw),
        })
    }

    fn write_buffer(&mut self, buffer: &Buffer, offset: u64, data: &[u8]) -> Result<(), String> {
        debug_assert!(buffer.write, "Cannot write data to a static buffer");

        // update inner buffer if the size is not enough
        if buffer.size() < data.len() {
            let required = offset as usize + data.len();
            let next_size = next_buffer_size(buffer.size(), required);

            log::info!(
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
                log::info!(
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
        let raw = self.ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: desc.label,
            address_mode_u: desc.wrap_x.as_wgpu(),
            address_mode_v: desc.wrap_y.as_wgpu(),
            address_mode_w: desc.wrap_z.as_wgpu(),
            mag_filter: desc.mag_filter.as_wgpu(),
            min_filter: desc.min_filter.as_wgpu(),
            mipmap_filter: desc
                .mipmap_filter
                .map_or(Default::default(), |tf| tf.as_wgpu()),
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
        let id = resource_id(&mut self.next_resource_id);
        create_texture(id, &self.ctx.device, &self.ctx.queue, desc, data)
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
        // Create the color texture
        let texture = self.create_texture(
            TextureDescriptor {
                label: Some("Create RenderTexture inner color texture"),
                format: desc.format,
                write: true,
            },
            Some(TextureData {
                bytes: &[],
                width: desc.width,
                height: desc.height,
            }),
        )?;

        // Create the depth texture
        let depth_texture = {
            let tex = desc.depth.then(|| {
                self.create_texture(
                    TextureDescriptor {
                        label: Some("Create RenderTexture inner color texture"),
                        format: TextureFormat::Depth32Float,
                        write: true,
                    },
                    Some(TextureData {
                        bytes: &[],
                        width: desc.width,
                        height: desc.height,
                    }),
                )
            });

            match tex {
                Some(Ok(t)) => Some(t),
                Some(Err(e)) => return Err(e),
                None => None,
            }
        };

        Ok(RenderTexture {
            id: resource_id(&mut self.next_resource_id),
            texture,
            depth_texture,
        })
    }

    fn limits(&self) -> Limits {
        let raw_limits = self.ctx.device.limits();
        Limits {
            max_texture_size_2d: raw_limits.max_texture_dimension_2d,
            max_texture_size_3d: raw_limits.max_texture_dimension_3d,
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
    async fn new<W>(
        window: &W,
        vsync: bool,
        win_size: UVec2,
        pixelated: bool,
    ) -> Result<Self, String>
    where
        W: HasWindowHandle + HasDisplayHandle,
    {
        let depth_format = SURFACE_DEFAULT_DEPTH_FORMAT; // make it configurable?
        let mut next_resource_id = 0;

        let instance = if cfg!(all(target_arch = "wasm32", feature = "webgl")) {
            Instance::new(&InstanceDescriptor {
                backends: Backends::GL,
                flags: InstanceFlags::default().with_env(),
                backend_options: BackendOptions {
                    gl: GlBackendOptions {
                        gles_minor_version: Gles3MinorVersion::from_env()
                            .unwrap_or(Gles3MinorVersion::Automatic),
                    },
                    dx12: Default::default(),
                },
            })
        } else {
            Instance::default()
        };

        let (ctx, surface) = {
            let raw = Surface::create_raw_surface(window, &instance)?;
            let mut ctx = Context::new(instance, Some(&raw)).await?;
            let surface = init_surface_from_raw(
                resource_id(&mut next_resource_id),
                &mut ctx,
                raw,
                depth_format,
                win_size,
                vsync,
            )
            .await?;
            (ctx, surface)
        };

        let mut bck = Self {
            next_resource_id,
            ctx,
            depth_format,
            surface,
            frame: None,
            offscreen: None,
            last_size: win_size,
            last_frame_stats: GpuStats::default(),
            current_stats: GpuStats::default(),
        };

        let offscreen = OffscreenSurfaceData::new(&mut bck, pixelated)?;
        bck.offscreen = Some(offscreen);

        Ok(bck)
    }

    fn push_frame(&mut self) -> Result<(), String> {
        match self.surface.frame() {
            Ok(frame) => {
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let encoder =
                    self.ctx
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Frame Encode"),
                        });
                self.frame = Some(DrawFrame {
                    frame,
                    view,
                    encoder,
                    dirty: false,
                });
            }
            Err(err) => match err {
                wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost => {
                    log::debug!("Resizng surface because: {err}");
                    self.resize(self.last_size.x, self.last_size.y);
                }
                e => {
                    return Err(e.to_string());
                }
            },
        };

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
        match self.frame.take() {
            Some(mut df) => {
                if df.dirty {
                    // TODO change this: "take" is ugly as hell
                    let offscreen = self.offscreen.take().unwrap();
                    offscreen.present(self, &mut df).unwrap();
                    self.offscreen = Some(offscreen);

                    let DrawFrame { frame, encoder, .. } = df;

                    self.ctx.queue.submit(Some(encoder.finish()));
                    self.current_stats.draw_calls += 1;
                    frame.present();
                }
            }
            _ => {
                log::debug!("Cannot find a frame to present. Skipping.");
            }
        }
    }

    #[inline]
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        let can_resize = width > 0 && height > 0;
        if !can_resize {
            return;
        }

        self.last_size = uvec2(width, height);

        self.surface.resize(&self.ctx.device, width, height);
        let mut offscreen = self.offscreen.take().unwrap();
        offscreen.update(self).unwrap();
        self.offscreen = Some(offscreen);
    }
}

async fn init_surface_from_raw(
    id: TextureId,
    ctx: &mut Context,
    raw: RawSurface<'static>,
    depth_format: TextureFormat,
    win_physical_size: UVec2,
    vsync: bool,
) -> Result<Surface, String> {
    let depth_texture = create_texture(
        id,
        &ctx.device,
        &ctx.queue,
        TextureDescriptor {
            label: Some("Depth Texture for Surface"),
            format: depth_format,
            write: true,
        },
        Some(TextureData {
            bytes: &[],
            width: win_physical_size.x,
            height: win_physical_size.y,
        }),
    )?;

    Surface::new_from_raw(ctx, raw, win_physical_size, vsync, depth_texture).await
}

fn create_texture(
    id: TextureId,
    device: &wgpu::Device,
    queue: &Queue,
    desc: TextureDescriptor,
    data: Option<TextureData>,
) -> Result<Texture, String> {
    let size = data.map_or(wgpu::Extent3d::default(), |d| wgpu::Extent3d {
        width: d.width,
        height: d.height,
        depth_or_array_layers: 1,
    });

    let is_depth_texture = matches!(desc.format, TextureFormat::Depth32Float);
    let mut usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    if is_depth_texture || desc.write {
        usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    }

    let raw = device.create_texture(&wgpu::TextureDescriptor {
        label: desc.label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: desc.format.as_wgpu(),
        usage,
        view_formats: &[],
    });

    if !is_depth_texture {
        if let Some(d) = data {
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
    pub fn as_wgpu(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }
}

fn next_buffer_size(current: usize, required: usize) -> usize {
    next_pot2(current.max(required))
}
