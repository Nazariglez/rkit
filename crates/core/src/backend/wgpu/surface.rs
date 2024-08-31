use crate::backend::wgpu::context::Context;
use crate::backend::wgpu::texture::{InnerTexture, Texture};
use crate::math::{UVec2, Vec2};
use std::sync::Arc;
use wgpu::rwh::HasDisplayHandle;
use wgpu::{
    Device, Surface as RawSurface, SurfaceCapabilities, SurfaceConfiguration, SurfaceTexture,
};
use winit::raw_window_handle::HasWindowHandle;

#[derive(Clone)]
pub(crate) struct Surface {
    pub surface: Arc<RawSurface<'static>>,
    pub config: SurfaceConfiguration,
    pub capabilities: Arc<SurfaceCapabilities>,
    pub depth_texture: InnerTexture,
}

impl Surface {
    pub async fn new<W>(
        ctx: &mut Context,
        window: &W,
        win_physical_size: UVec2,
        vsync: bool,
        depth_texture: InnerTexture,
    ) -> Result<Self, String>
    where
        W: HasDisplayHandle + HasWindowHandle,
    {
        log::trace!("Generating main surface");
        let surface = unsafe {
            ctx.instance.create_surface_unsafe(
                wgpu::SurfaceTargetUnsafe::from_window(window).map_err(|e| e.to_string())?,
            )
        }
        .map_err(|e| e.to_string())?;

        if !ctx.is_surface_compatible(&surface) {
            log::trace!("Generating WGPU adapter compatible surface.",);
            ctx.ensure_surface_compatibility(&surface).await?;
        }

        let UVec2 {
            x: width,
            y: height,
        } = win_physical_size;
        let capabilities = surface.get_capabilities(&ctx.adapter);
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: capabilities.formats[0],
            width,
            height,
            present_mode: if vsync {
                wgpu::PresentMode::AutoVsync
            } else {
                wgpu::PresentMode::AutoNoVsync
            },
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&ctx.device, &config);

        println!(
            "Surface size({:?} {:?}) depth_texture({:?})",
            config.width, config.height, depth_texture.size
        );

        Ok(Self {
            surface: Arc::new(surface),
            config,
            capabilities: Arc::new(capabilities),
            depth_texture,
        })
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(device, &self.config);
    }

    pub fn frame(&self) -> Result<SurfaceTexture, String> {
        self.surface
            .get_current_texture()
            .map_err(|e| e.to_string())
    }
}
