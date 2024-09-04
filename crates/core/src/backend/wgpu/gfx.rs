use crate::backend::backend::GfxBackendImpl;
use crate::backend::wgpu::context::Context;
use crate::backend::wgpu::frame::DrawFrame;
use crate::backend::wgpu::surface::Surface;
use crate::backend::wgpu::utils::{wgpu_depth_stencil, wgpu_shader_visibility};
use crate::gfx::consts::SURFACE_DEFAULT_DEPTH_FORMAT;
use crate::gfx::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutRef, BindType, Buffer,
    BufferDescriptor, BufferUsage, Color, RenderPipeline, RenderPipelineDescriptor, RenderTexture,
    Renderer, Texture, TextureData, TextureDescriptor, TextureFormat, TextureId,
};
use crate::gfx::{Sampler, SamplerDescriptor, MAX_BINDING_ENTRIES};
use crate::math::{vec2, UVec2, Vec2};
use arrayvec::ArrayVec;
use std::borrow::Cow;
use std::sync::Arc;
use wgpu::rwh::HasWindowHandle;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Color as WColor, Queue, StoreOp, TextureDimension};
use winit::raw_window_handle::HasDisplayHandle;

pub(crate) struct GfxBackend {
    next_resource_id: u64,
    vsync: bool,
    ctx: Context,
    depth_format: TextureFormat,
    surface: Surface, // Eventually we could have a HashMap<WindowId, Surface> if we want multiple window
    frame: Option<DrawFrame>,
}

// This is a hack for wasm32 browsers where there is no threads
#[cfg(target_arch = "wasm32")]
unsafe impl Send for GfxBackend {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for GfxBackend {}

impl GfxBackendImpl for GfxBackend {
    async fn init<W>(window: &W, vsync: bool, win_size: UVec2) -> Result<Self, String>
    where
        Self: Sized,
        W: HasDisplayHandle + HasWindowHandle,
    {
        Self::new(window, vsync, win_size).await
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
        let surface = init_surface(
            id,
            &mut self.ctx,
            window,
            self.depth_format,
            win_size,
            vsync,
        )
        .await?;
        self.surface = surface;
        Ok(())
    }

    fn prepare_frame(&mut self) {
        if let Err(e) = self.push_frame() {
            log::error!("Error creating frame: {}", e);
        }
    }

    fn present_frame(&mut self) {
        self.present_to_screen();
    }

    fn render(&mut self, renderer: &Renderer) -> Result<(), String> {
        let frame = self
            .frame
            .as_mut()
            .ok_or_else(|| "Unavailable frame".to_string())?;

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
                    |color| wgpu::RenderPassColorAttachment {
                        view: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: rp.clear_options.color.map_or(wgpu::LoadOp::Load, |color| {
                                wgpu::LoadOp::Clear(color.to_wgpu())
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
                            .map_or(wgpu::LoadOp::Load, |stencil| wgpu::LoadOp::Clear(stencil)),
                        store: StoreOp::Store,
                    })
                } else {
                    None
                };

                let mut encoder = &mut frame.encoder;
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
                    rp.buffers.iter().for_each(|buff| match buff.usage {
                        BufferUsage::Vertex => {
                            rpass.set_vertex_buffer(vertex_buffers_slot, buff.raw.slice(..));
                            vertex_buffers_slot += 1;
                        }
                        BufferUsage::Index => {
                            debug_assert!(!indexed, "Cannot bind more than one Index buffer");
                            indexed = true;
                            rpass.set_index_buffer(buff.raw.slice(..), pip.index_format)
                        }
                        BufferUsage::Uniform => {}
                    });

                    rp.bind_groups.iter().enumerate().for_each(|(i, bg)| {
                        rpass.set_bind_group(i as _, &bg.raw, &[]);
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

        // mark the frame as dirty if some work was made
        if !renderer.passes.is_empty() {
            frame.dirty = true;
        }

        Ok(())
    }

    fn render_to(&mut self, texture: &RenderTexture, renderer: &Renderer) -> Result<(), String> {
        todo!()
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

        // TODO is this right? I don't see issues if we keep formats the same for now
        let swapchain_format = self.surface.capabilities.formats[0];
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
                            format: attr.format.to_wgpu(),
                            offset,
                            shader_location: attr.location as _,
                        };
                        offset += a.format.size();
                        a
                    })
                    .collect::<Vec<_>>();

                let layout = wgpu::VertexBufferLayout {
                    array_stride: offset,
                    step_mode: vl.step_mode.to_wgpu(),
                    attributes: &[],
                };

                (attrs, layout)
            })
            .unzip();

        buffers
            .iter_mut()
            .enumerate()
            .for_each(|(i, buff)| buff.attributes = &attrs[i]);

        let swapchain_color_target: wgpu::ColorTargetState = swapchain_format.into();
        let color_target = wgpu::ColorTargetState {
            blend: desc.blend_mode.map(|bm| bm.to_wgpu()),
            write_mask: desc.color_mask.to_wgpu(),
            ..swapchain_color_target
        };

        let raw = self
            .ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: desc.label,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: desc.vs_entry.unwrap_or("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: desc.fs_entry.unwrap_or("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(color_target)],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: desc.primitive.to_wgpu(),
                    cull_mode: desc.cull_mode.map(|cm| cm.to_wgpu()),
                    ..Default::default()
                },
                depth_stencil: wgpu_depth_stencil(desc.depth_stencil, desc.stencil),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let index_format = desc.index_format.to_wgpu();
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
        let mut usage = desc.usage.to_wgpu();
        if desc.write {
            usage |= wgpu::BufferUsages::COPY_DST;
        }

        let raw = self.ctx.device.create_buffer_init(&BufferInitDescriptor {
            label: desc.label,
            contents: desc.content,
            usage,
        });

        let usage = desc.usage;
        let size = desc.content.len();

        Ok(Buffer {
            id: resource_id(&mut self.next_resource_id),
            raw: Arc::new(raw),
            usage,
            write: desc.write,
            size,
        })
    }

    fn create_bind_group(&mut self, desc: BindGroupDescriptor) -> Result<BindGroup, String> {
        let mut entries: ArrayVec<_, MAX_BINDING_ENTRIES> = Default::default();
        desc.entry.iter().for_each(|entry| match entry {
            BindGroupEntry::Texture { location, texture } => {
                entries.push(wgpu::BindGroupEntry {
                    binding: *location,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                });
            }
            BindGroupEntry::Uniform { location, buffer } => {
                entries.push(wgpu::BindGroupEntry {
                    binding: *location,
                    resource: buffer.raw.as_entire_binding(),
                });
            }
            BindGroupEntry::Sampler { location, sampler } => {
                entries.push(wgpu::BindGroupEntry {
                    binding: *location,
                    resource: wgpu::BindingResource::Sampler(&sampler.raw),
                });
            }
        });
        let raw = self
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: desc.label,
                layout: &desc
                    .layout
                    .ok_or_else(|| "Cannot create binding group with a missing layout.")?
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
        debug_assert!(
            buffer.len() <= offset as usize + data.len(),
            "Invalid buffer size '{}' expected '{}'",
            buffer.len(),
            offset as usize + data.len()
        );
        self.ctx.queue.write_buffer(&buffer.raw, offset as _, data);
        Ok(())
    }

    fn create_sampler(&mut self, desc: SamplerDescriptor) -> Result<Sampler, String> {
        let raw = self.ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: desc.label,
            address_mode_u: desc.wrap_x.to_wgpu(),
            address_mode_v: desc.wrap_y.to_wgpu(),
            address_mode_w: desc.wrap_z.to_wgpu(),
            mag_filter: desc.mag_filter.to_wgpu(),
            min_filter: desc.min_filter.to_wgpu(),
            mipmap_filter: desc
                .mipmap_filter
                .map_or(Default::default(), |tf| tf.to_wgpu()),
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
            mipmap_filter: desc.mipmap_filter,
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
}

#[inline(always)]
fn resource_id<T: From<u64>>(count: &mut u64) -> T {
    let id = *count;
    *count += 1;
    T::from(id)
}

impl GfxBackend {
    async fn new<W>(window: &W, vsync: bool, win_size: UVec2) -> Result<Self, String>
    where
        W: HasWindowHandle + HasDisplayHandle,
    {
        let depth_format = SURFACE_DEFAULT_DEPTH_FORMAT; // make it configurable?
        let mut ctx = Context::new().await?;
        let mut next_resource_id = 0;
        let surface = init_surface(
            resource_id(&mut next_resource_id),
            &mut ctx,
            window,
            depth_format,
            win_size,
            vsync,
        )
        .await?;
        Ok(Self {
            next_resource_id,
            vsync,
            ctx,
            depth_format,
            surface,
            frame: None,
        })
    }

    fn push_frame(&mut self) -> Result<(), String> {
        let frame = self.surface.frame()?;
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
            present_check: false,
        });

        Ok(())
    }

    fn present_to_screen(&mut self) {
        match self.frame.take() {
            None => {
                log::error!("Cannot find a frame to present.");
            }
            Some(df) => {
                let DrawFrame {
                    frame,
                    view,
                    encoder,
                    dirty,
                    present_check: _,
                } = df;

                if dirty {
                    self.ctx.queue.submit(Some(encoder.finish()));
                    frame.present();
                }
            }
        }
    }
}

async fn init_surface<W>(
    id: TextureId,
    ctx: &mut Context,
    window: &W,
    depth_format: TextureFormat,
    win_physical_size: UVec2,
    vsync: bool,
) -> Result<Surface, String>
where
    W: HasDisplayHandle + HasWindowHandle,
{
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

    Surface::new(ctx, window, win_physical_size, vsync, depth_texture).await
}

struct InnerTextureInfo {}

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

    let is_depth_texture = match desc.format {
        TextureFormat::Depth32Float => true,
        _ => false,
    };
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
        format: desc.format.to_wgpu(),
        usage,
        view_formats: &[],
    });

    if !is_depth_texture {
        if let Some(d) = data {
            if !d.bytes.is_empty() {
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &raw,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    d.bytes,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(d.width * 4),
                        rows_per_image: Some(d.height),
                    },
                    size,
                );
            }
        }
    }

    let view = raw.create_view(&wgpu::TextureViewDescriptor::default());

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
    pub fn to_wgpu(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }
}
