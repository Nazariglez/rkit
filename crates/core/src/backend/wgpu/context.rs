use wgpu::{Adapter, Device, Instance, PowerPreference, Queue, Surface as RawSurface};

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
        .ok_or_else(|| "Cannot create WGPU Adapter".to_string())?;

    log::debug!("Wgpu Adapter: {:?}", adapter.get_info());
    log::info!(
        "Gpu Adapter: {} - {}",
        adapter.get_info().backend,
        adapter.get_info().name
    );

    let limits = adapter.limits();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::default(),
                required_limits: limits,
                memory_hints: Default::default(),
            },
            None,
        )
        .await
        .map_err(|err| err.to_string())?;

    log::debug!("WGPU Features {:?}", device.features());

    Ok((adapter, device, queue))
}
