use std::sync::{Arc, Mutex};

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
    pub supports_float32_filtering: bool,
    pub supports_compute: bool,
    device_loss: Arc<Mutex<Option<String>>>,
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

        let info = adapter.get_info();
        log::debug!("Wgpu Adapter: {info:?}");
        log::info!(
            "GPU Adapter: {} - {} ({}: {})",
            info.backend,
            info.name,
            info.driver,
            info.driver_info
        );

        let supports_view_formats = adapter
            .get_downlevel_capabilities()
            .flags
            .contains(DownlevelFlags::VIEW_FORMATS);
        let limits = adapter.limits();
        let supports_compute = !matches!(info.backend, wgpu::Backend::Gl)
            && limits.max_compute_workgroups_per_dimension > 0
            && limits.max_compute_invocations_per_workgroup > 0;
        let supports_float32_filtering = adapter
            .features()
            .contains(wgpu::Features::FLOAT32_FILTERABLE);
        let required_features = supports_float32_filtering
            .then_some(wgpu::Features::FLOAT32_FILTERABLE)
            .unwrap_or_default();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features,
                required_limits: limits,
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                experimental_features: ExperimentalFeatures::default(),
            })
            .await
            .map_err(|err| err.to_string())?;

        let device_loss = Arc::new(Mutex::new(None));
        let device_loss_callback = device_loss.clone();
        device.set_device_lost_callback(move |reason, message| {
            let error = format!("WGPU device lost ({reason:?}): {message}");
            log::error!("{error}");
            if let Ok(mut loss) = device_loss_callback.lock() {
                *loss = Some(error);
            }
        });
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
            supports_float32_filtering,
            supports_compute,
            device_loss,
        })
    }

    pub fn is_surface_compatible(&self, surface: &RawSurface) -> bool {
        self.adapter.is_surface_supported(surface)
    }

    pub(crate) fn device_loss(&self) -> Option<String> {
        self.device_loss.lock().ok().and_then(|loss| loss.clone())
    }
}
