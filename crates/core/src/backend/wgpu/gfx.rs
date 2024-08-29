use crate::backend::backend::GfxBackendImpl;
use crate::backend::wgpu::context::Context;
use crate::backend::wgpu::frame::DrawFrame;
use crate::backend::wgpu::surface::Surface;
use crate::backend::wgpu::texture::{InnerTexture, Texture};
use crate::gfx::{
    Color, RenderTexture, Renderer, TextureData, TextureDescriptor, TextureFormat, TextureId,
};
use crate::math::{vec2, UVec2, Vec2};
use std::sync::Arc;
use wgpu::rwh::HasWindowHandle;
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
    fn init<W>(window: &W, vsync: bool, win_size: UVec2) -> Result<Self, String>
    where
        Self: Sized,
        W: HasDisplayHandle + HasWindowHandle,
    {
        Self::new(window, vsync, win_size)
    }

    fn update_surface<W>(&mut self, window: &W, vsync: bool, win_size: UVec2) -> Result<(), String>
    where
        Self: Sized,
        W: HasDisplayHandle + HasWindowHandle,
    {
        let surface = init_surface(&mut self.ctx, window, self.depth_format, win_size, vsync)?;
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
                // let (uses_depth, uses_stencil) = rp
                //     .pipeline
                //     .map_or((false, false), |pip| (pip.uses_depth, pip.uses_stencil));

                let (uses_depth, uses_stencil) = (false, false);

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
                        // Some(wgpu::RenderPassDepthStencilAttachment {
                        //     view: &frame.surface.depth_texture.view,
                        //     depth_ops: depth,
                        //     stencil_ops: stencil,
                        // })
                        None // TODO
                    } else {
                        None
                    },
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                /* if let Some(pip) = rp.pipeline {
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
                }*/

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
}

impl GfxBackend {
    fn new<W>(window: &W, vsync: bool, win_size: UVec2) -> Result<Self, String>
    where
        W: HasWindowHandle + HasDisplayHandle,
    {
        let depth_format = TextureFormat::Depth24Stencil8; // make it configurable?
        let mut ctx = Context::new()?;
        let surface = init_surface(&mut ctx, window, depth_format, win_size, vsync)?;
        Ok(Self {
            next_resource_id: 0,
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

fn init_surface<W>(
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

    Surface::new(ctx, window, win_physical_size, vsync, depth_texture)
}

struct InnerTextureInfo {}

fn create_texture(
    device: &wgpu::Device,
    queue: &Queue,
    desc: TextureDescriptor,
    data: Option<TextureData>,
) -> Result<InnerTexture, String> {
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

    Ok(InnerTexture {
        raw: Arc::new(raw),
        view: Arc::new(view),
        size: vec2(size.width as _, size.height as _),
        write: desc.write,
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
