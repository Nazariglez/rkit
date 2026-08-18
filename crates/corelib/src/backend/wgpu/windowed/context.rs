use std::sync::Arc;

use wgpu::{
    Adapter, Device, DownlevelFlags, ExperimentalFeatures, Instance, PowerPreference, Queue,
    Surface as RawSurface,
};

pub(crate) struct Context {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub supports_view_formats: bool,
}

impl Context {
    pub(crate) async fn new(
        instance: Instance,
        surface: &RawSurface<'static>,
    ) -> Result<Self, String> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("Cannot create WGPU Adapter: {e}"))?;

        log::debug!("Wgpu Adapter: {:?}", adapter.get_info());
        log::info!(
            "GPU Adapter: {} - {} ({}: {})",
            adapter.get_info().backend,
            adapter.get_info().name,
            adapter.get_info().driver,
            adapter.get_info().driver_info
        );

        let supports_view_formats = adapter
            .get_downlevel_capabilities()
            .flags
            .contains(DownlevelFlags::VIEW_FORMATS);
        let limits = adapter.limits();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::default(),
                required_limits: limits,
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                experimental_features: ExperimentalFeatures::default(),
            })
            .await
            .map_err(|err| err.to_string())?;

        device.on_uncaptured_error(Arc::new(|e| {
            eprintln!("WGPU Error: {e}");
            log::error!("WGPU Error: {e}");

            if cfg!(debug_assertions) {
                panic!("WGPU Error: {e}");
            }
        }));
        log::debug!("WGPU Features {:?}", device.features());

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            supports_view_formats,
        })
    }

    pub fn is_surface_compatible(&self, surface: &RawSurface) -> bool {
        self.adapter.is_surface_supported(surface)
    }
}
