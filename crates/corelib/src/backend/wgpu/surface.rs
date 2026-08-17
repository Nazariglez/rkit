use crate::backend::traits::SurfaceSource;
use crate::backend::wgpu::context::Context;
use crate::backend::wgpu::texture::Texture;
use crate::math::UVec2;
use wgpu::{
    CurrentSurfaceTexture, Device, Instance, PresentMode, Surface as RawSurface,
    SurfaceCapabilities, SurfaceConfiguration,
};

pub(crate) struct SurfaceOwner {
    surface: RawSurface<'static>,
    source: SurfaceSource,
}

impl SurfaceOwner {
    pub fn new(source: SurfaceSource, instance: &Instance) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let surface = {
            let target = wgpu::SurfaceTarget::from_window_without_display(source.window.clone());
            instance.create_surface(target).map_err(|e| e.to_string())?
        };
        #[cfg(target_arch = "wasm32")]
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(source.canvas.clone()))
            .map_err(|e| e.to_string())?;

        Ok(Self { surface, source })
    }

    pub fn raw(&self) -> &RawSurface<'static> {
        &self.surface
    }
}

pub(crate) struct SurfaceCandidate {
    owner: SurfaceOwner,
    config: SurfaceConfiguration,
    capabilities: SurfaceCapabilities,
}

impl SurfaceCandidate {
    pub fn initial(
        ctx: &Context,
        owner: SurfaceOwner,
        size: UVec2,
        vsync: bool,
    ) -> Result<Self, String> {
        let capabilities = validate_surface(ctx, &owner)?;
        let format = capabilities
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .or_else(|| capabilities.formats.first())
            .copied()
            .ok_or_else(|| "Surface has no supported formats".to_string())?;
        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| "Surface has no supported alpha modes".to_string())?;
        let present_mode = present_mode(vsync);

        Ok(Self {
            owner,
            config: SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.x,
                height: size.y,
                present_mode,
                desired_maximum_frame_latency: 2,
                alpha_mode,
                view_formats: vec![],
                color_space: wgpu::SurfaceColorSpace::Auto,
            },
            capabilities,
        })
    }

    #[cfg(all(not(target_arch = "wasm32"), not(feature = "headless")))]
    pub fn replacement(
        ctx: &Context,
        owner: SurfaceOwner,
        size: UVec2,
        current: &Surface,
    ) -> Result<Self, String> {
        let capabilities = validate_surface(ctx, &owner)?;
        let config = &current.config;
        validate_committed_config(&capabilities, config, "Replacement")?;

        let mut config = config.clone();
        config.width = size.x;
        config.height = size.y;
        Ok(Self {
            owner,
            config,
            capabilities,
        })
    }

    pub fn configure(self, device: &Device, depth_texture: Texture) -> Surface {
        self.owner.raw().configure(device, &self.config);
        log::debug!(
            "Surface size({:?} {:?}), depth_texture({:?}), format({:?})",
            self.config.width,
            self.config.height,
            depth_texture.size,
            self.config.format,
        );

        Surface {
            owner: self.owner,
            config: self.config,
            depth_texture,
            capabilities: self.capabilities,
        }
    }
}

pub(crate) struct Surface {
    owner: SurfaceOwner,
    pub config: SurfaceConfiguration,
    pub depth_texture: Texture,
    pub capabilities: SurfaceCapabilities,
}

impl Surface {
    pub fn recreate(&mut self, ctx: &Context) -> Result<(), String> {
        let owner = SurfaceOwner::new(self.owner.source.clone(), &ctx.instance)
            .map_err(|e| format!("Cannot create replacement WGPU surface: {e}"))?;
        let capabilities = validate_surface(ctx, &owner)?;
        validate_committed_config(&capabilities, &self.config, "Recreated")?;
        owner.raw().configure(&ctx.device, &self.config);

        self.owner = owner;
        self.capabilities = capabilities;
        Ok(())
    }

    pub fn configure_size(&mut self, device: &Device, size: UVec2) {
        let mut config = self.config.clone();
        config.width = size.x;
        config.height = size.y;
        self.owner.raw().configure(device, &config);
        self.config = config;
    }

    pub fn reconfigure(&self, device: &Device) {
        self.owner.raw().configure(device, &self.config);
    }

    #[inline]
    pub fn frame(&self) -> CurrentSurfaceTexture {
        self.owner.raw().get_current_texture()
    }
}

fn validate_surface(ctx: &Context, owner: &SurfaceOwner) -> Result<SurfaceCapabilities, String> {
    if !ctx.is_surface_compatible(owner.raw()) {
        return Err("Surface is not compatible with the existing WGPU adapter".to_string());
    }

    let capabilities = owner.raw().get_capabilities(&ctx.adapter);
    log::debug!("Surface formats: {:?}", capabilities.formats);
    Ok(capabilities)
}

fn validate_committed_config(
    capabilities: &SurfaceCapabilities,
    config: &SurfaceConfiguration,
    operation: &str,
) -> Result<(), String> {
    if !capabilities.formats.contains(&config.format) {
        return Err(format!(
            "{operation} surface does not support the current format {:?}",
            config.format
        ));
    }
    if !capabilities.alpha_modes.contains(&config.alpha_mode) {
        return Err(format!(
            "{operation} surface does not support the current alpha mode {:?}",
            config.alpha_mode
        ));
    }
    if !supports_present_mode(capabilities, config.present_mode) {
        return Err(format!(
            "{operation} surface does not support the current present mode {:?}",
            config.present_mode
        ));
    }
    Ok(())
}

fn present_mode(vsync: bool) -> PresentMode {
    if vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    }
}

fn supports_present_mode(capabilities: &SurfaceCapabilities, mode: PresentMode) -> bool {
    matches!(mode, PresentMode::AutoVsync | PresentMode::AutoNoVsync)
        || capabilities.present_modes.contains(&mode)
}
