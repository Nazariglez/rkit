#![allow(clippy::unused_unit)]

use rustc_hash::FxHashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    pub fn console_error(s: &str);
}

/// Configure the logs output
/// Logs will show a timestamp using the UTC time with format `[year]-[month]-[day] [hour]:[minutes]:[seconds]`
#[derive(Clone)]
pub struct LogConfig {
    level: log::LevelFilter,
    levels_for: FxHashMap<String, log::LevelFilter>,
    colored: bool,
    verbose: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        let level = if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Warn
        };

        Self {
            level,
            levels_for: Default::default(),
            colored: cfg!(debug_assertions),
            verbose: false,
        }
    }
}

impl LogConfig {
    /// Creates a new configuration using the given level filter
    pub fn new(level: log::LevelFilter) -> Self {
        Self {
            level,
            ..Default::default()
        }
    }

    /// Configure logs to use trace level filter
    pub fn trace() -> Self {
        Self::new(log::LevelFilter::Trace)
    }

    /// Configure logs to use debug level filter
    pub fn debug() -> Self {
        Self::new(log::LevelFilter::Debug)
    }

    /// Configure logs to use info level filter
    pub fn info() -> Self {
        Self::new(log::LevelFilter::Info)
    }

    /// Configure logs to use warn level filter
    pub fn warn() -> Self {
        Self::new(log::LevelFilter::Warn)
    }

    /// Configure logs to use error level filter
    pub fn error() -> Self {
        Self::new(log::LevelFilter::Error)
    }

    /// Changes the level filter
    pub fn level(mut self, level: log::LevelFilter) -> Self {
        self.level = level;
        self
    }

    /// Change the filter level for dependencies
    pub fn level_for(mut self, id: &str, level: log::LevelFilter) -> Self {
        self.levels_for.insert(id.to_string(), level);
        self
    }

    /// Enable colored text (Defaults to true on debug mode)
    pub fn use_colors(mut self, value: bool) -> Self {
        self.colored = value;
        self
    }

    /// Log everything including dependencies
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_time() -> String {
    let format =
        time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
    time::OffsetDateTime::now_utc().format(&format).unwrap()
}

#[cfg(target_arch = "wasm32")]
fn get_time() -> String {
    let now = js_sys::Date::new_0();
    format!(
        "{}-{}-{} {}:{}:{}",
        now.get_utc_full_year(),
        now.get_utc_month() + 1,
        now.get_utc_date(),
        now.get_utc_hours(),
        now.get_utc_minutes(),
        now.get_utc_seconds()
    )
}

#[cfg(target_arch = "wasm32")]
fn chain_output(dispatch: fern::Dispatch) -> fern::Dispatch {
    dispatch.chain(fern::Output::call(console_log::log))
}

#[cfg(not(target_arch = "wasm32"))]
fn chain_output(dispatch: fern::Dispatch) -> fern::Dispatch {
    dispatch.chain(std::io::stdout())
}

#[cfg(target_arch = "wasm32")]
fn print_apply_error(e: &str) {
    console_error(&format!("Error initializing logs: {e}"));
}

#[cfg(not(target_arch = "wasm32"))]
fn print_apply_error(e: &str) {
    println!("Error initializing logs: {e}");
}

#[cfg(target_arch = "wasm32")]
fn set_panic_hook() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[cfg(not(target_arch = "wasm32"))]
fn set_panic_hook() {
    use std::panic::{self, PanicHookInfo};

    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info: &PanicHookInfo| {
        panic_to_log_error_hook(info);
        default_hook(info);
    }));
}

fn panic_to_log_error_hook(info: &std::panic::PanicHookInfo) {
    let payload = if let Some(payload) = info.payload().downcast_ref::<&str>() {
        payload
    } else if let Some(payload) = info.payload().downcast_ref::<String>() {
        payload.as_str()
    } else {
        "Unknown"
    };

    match info.location() {
        Some(location) => log::error!(
            "Panic at '{}:{}': {payload}",
            location.file(),
            location.line()
        ),
        None => log::error!("Panic: {payload}"),
    }
}

pub(crate) fn init_logs(mut config: LogConfig) {
    set_panic_hook();

    if !config.verbose {
        let mut disabled = vec![
            "cosmic_text",
            "symphonia_core",
            "symphonia_codec_vorbis",
            "symphonia_format_ogg",
            "wgpu_core",
            "wgpu_hal",
            "naga",
        ];

        if !cfg!(target_arch = "wasm32") {
            disabled.push("winit");
        }

        disabled.iter().for_each(|id| {
            config
                .levels_for
                .insert(id.to_string(), log::LevelFilter::Warn);
        });
    }

    let mut dispatch = fern::Dispatch::new().level(config.level);

    for (id, lvl) in config.levels_for.iter() {
        dispatch = dispatch.level_for(id.clone(), *lvl);
    }

    dispatch = chain_output(dispatch);

    if config.colored {
        use fern::colors::{Color, ColoredLevelConfig};

        let color_level = ColoredLevelConfig::new()
            .error(Color::BrightRed)
            .warn(Color::BrightYellow)
            .info(Color::BrightGreen)
            .debug(Color::BrightCyan)
            .trace(Color::BrightBlack);

        dispatch = dispatch.format(move |out, message, record| {
            out.finish(format_args!(
                "\x1b[0m{date} [{target}] {level}: {message}",
                date = get_time(),
                target = record.target(),
                level = format_args!(
                    "{}\x1b[{}m",
                    color_level.color(record.level()),
                    Color::White.to_fg_str()
                ),
                message = message,
            ))
        });
    } else {
        dispatch = dispatch.format(move |out, message, record| {
            out.finish(format_args!(
                "{date} [{target}] {level}: {message}",
                date = get_time(),
                target = record.target(),
                level = record.level(),
                message = message,
            ))
        });
    }

    if let Err(e) = dispatch.apply() {
        print_apply_error(&e.to_string());
    }
}
