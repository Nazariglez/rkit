use std::sync::Arc;

use wgpu::{
    Adapter, Device, ExperimentalFeatures, Instance, PowerPreference, Queue, Surface as RawSurface,
};

pub(crate) struct Context {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl Context {
    pub(crate) async fn new(
        instance: Instance,
        surface: Option<&RawSurface<'static>>,
    ) -> Result<Self, String> {
        let (adapter, device, queue) = generate_wgpu_ctx(&instance, surface).await?;
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    pub fn is_surface_compatible(&self, surface: &RawSurface) -> bool {
        self.adapter.is_surface_supported(surface)
    }

    pub async fn ensure_surface_compatibility(
        &mut self,
        surface: &RawSurface<'_>,
    ) -> Result<(), String> {
        let (adapter, device, queue) = generate_wgpu_ctx(&self.instance, Some(surface)).await?;
        self.adapter = adapter;
        self.device = device;
        self.queue = queue;
        Ok(())
    }
}

async fn generate_wgpu_ctx(
    instance: &Instance,
    surface: Option<&RawSurface<'_>>,
) -> Result<(Adapter, Device, Queue), String> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: surface,
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

    Ok((adapter, device, queue))
}
