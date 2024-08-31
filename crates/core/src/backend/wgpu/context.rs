use wgpu::{Adapter, Device, Instance, PowerPreference, Queue, Surface as RawSurface};

pub(crate) struct Context {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl Context {
    pub(crate) async fn new() -> Result<Self, String> {
        let instance = Instance::default();
        let (adapter, device, queue) = generate_wgpu_ctx(&instance, None).await?;
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

#[cfg(target_arch = "wasm32")]
fn generate_inner_ctx(
    instance: &Instance,
    surface: Option<&RawSurface<'_>>,
) -> Result<(Adapter, Device, Queue), String> {
    todo!()
    // wasm_bindgen_futures::spawn_local(generate_wgpu_ctx(instance, surface))
}

#[cfg(not(target_arch = "wasm32"))]
fn generate_inner_ctx(
    instance: &Instance,
    surface: Option<&RawSurface<'_>>,
) -> Result<(Adapter, Device, Queue), String> {
    pollster::block_on(generate_wgpu_ctx(instance, surface))
}

async fn generate_wgpu_ctx(
    instance: &Instance,
    surface: Option<&RawSurface<'_>>,
) -> Result<(Adapter, Device, Queue), String> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: surface,
        })
        .await
        .ok_or_else(|| "Cannot create WGPU Adapter for {:?}".to_string())?;

    // TODO depending on adapter here, require limits for it.
    let limits = if cfg!(all(target_arch = "wasm32", feature = "webgl")) {
        wgpu::Limits::downlevel_webgl2_defaults()
    } else {
        wgpu::Limits::default()
    };

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

    Ok((adapter, device, queue))
}
