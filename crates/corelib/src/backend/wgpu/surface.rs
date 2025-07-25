#![allow(clippy::arc_with_non_send_sync)]

use crate::backend::wgpu::context::Context;
use crate::backend::wgpu::texture::Texture;
use crate::math::UVec2;
use std::sync::Arc;
use wgpu::rwh::HasDisplayHandle;
use wgpu::{
    Device, Instance, Surface as RawSurface, SurfaceConfiguration, SurfaceError, SurfaceTexture, TextureFormat as RawTextureFormat
};
use winit::raw_window_handle::HasWindowHandle;

#[derive(Clone)]
pub(crate) struct Surface {
    pub surface: Arc<RawSurface<'static>>,
    pub config: SurfaceConfiguration,
    pub depth_texture: Texture,
    pub raw_format: RawTextureFormat,
    // pub capabilities: Arc<SurfaceCapabilities>,
}

impl Surface {
    #[inline]
    pub fn create_raw_surface<W>(
        window: &W,
        instance: &Instance,
    ) -> Result<RawSurface<'static>, String>
    where
        W: HasDisplayHandle + HasWindowHandle,
    {
        unsafe {
            instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_window(window).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())
        }
    }

    pub async fn new_from_raw(
        ctx: &mut Context,
        surface: RawSurface<'static>,
        win_physical_size: UVec2,
        vsync: bool,
        depth_texture: Texture,
    ) -> Result<Self, String> {
        if !ctx.is_surface_compatible(&surface) {
            log::trace!("Generating WGPU adapter compatible surface.");
            ctx.ensure_surface_compatibility(&surface).await?;
        }

        let UVec2 {
            x: width,
            y: height,
        } = win_physical_size;
        let capabilities = surface.get_capabilities(&ctx.adapter);

        log::debug!("Surface formats: {:?}", capabilities.formats);

        let raw_format = capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: raw_format,
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

        log::debug!(
            "Surface size({:?} {:?}), depth_texture({:?}), format({raw_format:?})",
            config.width,
            config.height,
            depth_texture.size,
        );

        Ok(Self {
            surface: Arc::new(surface),
            config,
            depth_texture,
            raw_format,
            // capabilities: Arc::new(capabilities),
        })
    }

    #[inline]
    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        debug_assert!(width > 0 && height > 0);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(device, &self.config);
    }

    #[inline]
    pub fn frame(&self) -> Result<SurfaceTexture, SurfaceError> {
        self.surface
            .get_current_texture()
    }
}
