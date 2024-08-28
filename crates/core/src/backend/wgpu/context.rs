use wgpu::{Adapter, Device, Instance, PowerPreference, Queue, Surface as RawSurface};

pub(crate) struct Context {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl Context {
    pub(crate) fn new() -> Result<Self, String> {
        let instance = Instance::default();
        let (adapter, device, queue) = pollster::block_on(generate_inner_ctx(&instance, None))?;
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

    pub fn ensure_surface_compatibility(&mut self, surface: &RawSurface) -> Result<(), String> {
        let (adapter, device, queue) =
            pollster::block_on(generate_inner_ctx(&self.instance, Some(surface)))?;
        self.adapter = adapter;
        self.device = device;
        self.queue = queue;
        Ok(())
    }
}

async fn generate_inner_ctx(
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
