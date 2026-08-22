use windows::Win32::{
    Devices::Display::{
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_MODE_INFO_TYPE_TARGET, DISPLAYCONFIG_PATH_INFO,
        DISPLAYCONFIG_RATIONAL, DISPLAYCONFIG_SOURCE_DEVICE_NAME, DisplayConfigGetDeviceInfo,
        GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
    },
    Foundation::ERROR_INSUFFICIENT_BUFFER,
    Graphics::Gdi::{
        DISPLAYCONFIG_PATH_MODE_IDX_INVALID, GetMonitorInfoW, HMONITOR, MONITORINFO, MONITORINFOEXW,
    },
};
use winit::{
    monitor::MonitorHandle,
    window::{Fullscreen, Window},
};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum MonitorFpsSource {
    WindowsActive,
    ExclusiveVideoMode,
    WinitFallback,
}

impl MonitorFpsSource {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::WindowsActive => "windows-active",
            Self::ExclusiveVideoMode => "exclusive-video-mode",
            Self::WinitFallback => "winit-fallback",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct MonitorRefresh {
    pub(super) hz: f64,
    pub(super) source: MonitorFpsSource,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum WindowMode {
    Windowed,
    Borderless,
    Exclusive,
}

impl WindowMode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Windowed => "windowed",
            Self::Borderless => "borderless",
            Self::Exclusive => "exclusive",
        }
    }
}

pub(super) struct MonitorProbe {
    pub(super) refresh: Option<MonitorRefresh>,
    pub(super) window_mode: WindowMode,
    pub(super) native_error: Option<String>,
}

impl MonitorProbe {
    pub(super) fn pacing_eq(&self, other: &Self) -> bool {
        self.refresh == other.refresh && self.window_mode == other.window_mode
    }
}

pub(super) fn probe(win: Option<&Window>) -> MonitorProbe {
    let Some(win) = win else {
        return MonitorProbe {
            refresh: None,
            window_mode: WindowMode::Windowed,
            native_error: None,
        };
    };

    let fullscreen = win.fullscreen();
    let window_mode = match &fullscreen {
        None => WindowMode::Windowed,
        Some(Fullscreen::Borderless(_)) => WindowMode::Borderless,
        Some(Fullscreen::Exclusive(_)) => WindowMode::Exclusive,
    };

    if let Some(Fullscreen::Exclusive(video_mode)) = fullscreen {
        let refresh =
            valid_millihertz(video_mode.refresh_rate_millihertz()).map(|hz| MonitorRefresh {
                hz,
                source: MonitorFpsSource::ExclusiveVideoMode,
            });
        return MonitorProbe {
            refresh,
            window_mode,
            native_error: None,
        };
    }

    let Some(monitor) = win.current_monitor() else {
        return MonitorProbe {
            refresh: None,
            window_mode,
            native_error: None,
        };
    };

    match native_monitor_fps(&monitor) {
        Ok(hz) => MonitorProbe {
            refresh: Some(MonitorRefresh {
                hz,
                source: MonitorFpsSource::WindowsActive,
            }),
            window_mode,
            native_error: None,
        },
        Err(error) => {
            let refresh = monitor
                .refresh_rate_millihertz()
                .and_then(valid_millihertz)
                .map(|hz| MonitorRefresh {
                    hz,
                    source: MonitorFpsSource::WinitFallback,
                });
            MonitorProbe {
                refresh,
                window_mode,
                native_error: Some(error),
            }
        }
    }
}

fn valid_millihertz(millihertz: u32) -> Option<f64> {
    valid_hz(millihertz as f64 / 1_000.0)
}

fn valid_hz(hz: f64) -> Option<f64> {
    (hz.is_finite() && hz > 0.0).then_some(hz)
}

fn native_monitor_fps(monitor: &MonitorHandle) -> Result<f64, String> {
    let monitor_name = monitor_gdi_name(monitor)?;
    let (paths, modes) = active_display_config()?;
    let mut matched = None;

    for path in &paths {
        let source_name = source_gdi_name(path)?;
        if !source_name.eq_ignore_ascii_case(&monitor_name) {
            continue;
        }
        if matched.is_some() {
            return Err(format!(
                "multiple active display paths match {monitor_name}"
            ));
        }
        matched = Some(path);
    }

    let path = matched.ok_or_else(|| format!("no active display path matches {monitor_name}"))?;
    path_refresh(path, &modes)
        .ok_or_else(|| format!("active display path for {monitor_name} has no valid refresh rate"))
}

fn monitor_gdi_name(monitor: &MonitorHandle) -> Result<String, String> {
    use std::ffi::c_void;
    use winit::platform::windows::MonitorHandleExtWindows;

    let hmonitor = HMONITOR(monitor.hmonitor() as *mut c_void);
    let mut monitor_info = MONITORINFOEXW::default();
    monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    let succeeded = unsafe {
        GetMonitorInfoW(
            hmonitor,
            (&mut monitor_info as *mut MONITORINFOEXW).cast::<MONITORINFO>(),
        )
    };
    if !succeeded.as_bool() {
        return Err(format!(
            "GetMonitorInfoW failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    wide_string(&monitor_info.szDevice, "monitor GDI name")
}

fn active_display_config()
-> Result<(Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>), String> {
    for _ in 0..3 {
        let mut path_count = 0;
        let mut mode_count = 0;
        let status = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        };
        if status == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if status.0 != 0 {
            return Err(format!(
                "GetDisplayConfigBufferSizes failed with error {}",
                status.0
            ));
        }
        if path_count == 0 {
            return Err("Windows returned no active display paths".to_string());
        }

        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = display_config_vec(path_count)?;
        let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = display_config_vec(mode_count)?;
        let status = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };
        if status == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if status.0 != 0 {
            return Err(format!("QueryDisplayConfig failed with error {}", status.0));
        }

        let returned_paths = usize::try_from(path_count)
            .map_err(|_| "Windows returned an invalid display path count".to_string())?;
        let returned_modes = usize::try_from(mode_count)
            .map_err(|_| "Windows returned an invalid display mode count".to_string())?;
        if returned_paths > paths.len() || returned_modes > modes.len() {
            return Err("Windows returned display counts larger than the buffers".to_string());
        }
        paths.truncate(returned_paths);
        modes.truncate(returned_modes);
        return Ok((paths, modes));
    }

    Err("display topology changed during all three query attempts".to_string())
}

fn display_config_vec<T: Clone + Default>(count: u32) -> Result<Vec<T>, String> {
    let count = usize::try_from(count)
        .map_err(|_| "Windows returned an invalid display buffer count".to_string())?;
    let mut items = Vec::new();
    items
        .try_reserve_exact(count)
        .map_err(|err| format!("cannot allocate Windows display buffer: {err}"))?;
    items.resize(count, T::default());
    Ok(items)
}

fn source_gdi_name(path: &DISPLAYCONFIG_PATH_INFO) -> Result<String, String> {
    let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
    source.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
        size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
        adapterId: path.sourceInfo.adapterId,
        id: path.sourceInfo.id,
    };
    let request = (&mut source as *mut DISPLAYCONFIG_SOURCE_DEVICE_NAME)
        .cast::<DISPLAYCONFIG_DEVICE_INFO_HEADER>();
    let status = unsafe { DisplayConfigGetDeviceInfo(request) };
    if status != 0 {
        return Err(format!(
            "DisplayConfigGetDeviceInfo failed with error {status}"
        ));
    }
    wide_string(&source.viewGdiDeviceName, "display source GDI name")
}

fn path_refresh(path: &DISPLAYCONFIG_PATH_INFO, modes: &[DISPLAYCONFIG_MODE_INFO]) -> Option<f64> {
    if !path.targetInfo.targetAvailable.as_bool() {
        return None;
    }

    let mode_index = unsafe { path.targetInfo.Anonymous.modeInfoIdx };
    if mode_index != DISPLAYCONFIG_PATH_MODE_IDX_INVALID {
        let mode = usize::try_from(mode_index)
            .ok()
            .and_then(|index| modes.get(index));
        if let Some(mode) = mode {
            let is_target = mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_TARGET
                && mode.adapterId == path.targetInfo.adapterId
                && mode.id == path.targetInfo.id;
            if is_target {
                let target_mode = unsafe { mode.Anonymous.targetMode };
                if let Some(hz) = rational_hz(target_mode.targetVideoSignalInfo.vSyncFreq) {
                    return Some(hz);
                }
            }
        }
    }

    rational_hz(path.targetInfo.refreshRate)
}

fn rational_hz(rate: DISPLAYCONFIG_RATIONAL) -> Option<f64> {
    if rate.Numerator == 0 || rate.Denominator == 0 {
        return None;
    }
    valid_hz(rate.Numerator as f64 / rate.Denominator as f64)
}

fn wide_string(wide: &[u16], label: &str) -> Result<String, String> {
    let len = wide.iter().position(|ch| *ch == 0).unwrap_or(wide.len());
    if len == 0 {
        return Err(format!("Windows returned an empty {label}"));
    }
    String::from_utf16(&wide[..len])
        .map_err(|err| format!("Windows returned an invalid {label}: {err}"))
}
