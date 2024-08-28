use crate::backend::wgpu::context::Context;
use crate::backend::wgpu::surface::Surface;
use crate::backend::wgpu::texture::{InnerTexture, Texture};
use crate::gfx::{TextureData, TextureDescriptor, TextureFormat, TextureId};
use glam::{vec2, Vec2};
use std::sync::Arc;
use wgpu::{Device, Queue, TextureDimension};

pub(crate) struct GfxBackend {
    next_resource_id: u64,
    vsync: bool,
    ctx: Context,
    depth_format: TextureFormat,
    surface: Surface, // Eventually we could have a HashMap<WindowId, Surface> if we want multiple window
}

impl GfxBackend {
    fn new(vsync: bool) -> Self {
        let ctx = Context::new().unwrap();
    }
}

fn init_surface(
    device: &Device,
    queue: &Queue,
    depth_format: TextureFormat,
    win_physical_size: Vec2,
) -> Result<(), String> {
    let depth_texture = create_texture(
        device,
        queue,
        TextureDescriptor {
            label: Some("Depth Texture for Surface"),
            format: depth_format,
            write: true,
        },
        Some(TextureData {
            bytes: &[],
            width: win_physical_size.x as _,
            height: win_physical_size.y as _,
        }),
    )?;

    let surface = Surface::new(&mut self.ctx, window, self.attrs, depth_texture)?;
    self.surfaces.insert(window.id(), surface);

    Ok(())
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
