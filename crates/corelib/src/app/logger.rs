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

const LOG_FILE_EXT: &str = "log";

/// Configure the logs output
/// Logs will show a timestamp using the UTC time with format `[year]-[month]-[day] [hour]:[minutes]:[seconds]`
#[derive(Clone)]
pub struct LogConfig {
    level: log::LevelFilter,
    levels_for: FxHashMap<String, log::LevelFilter>,
    colored: bool,
    verbose: bool,
    log_files_path: Option<std::path::PathBuf>,
    log_files_format: String,
    max_log_files: Option<usize>,
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
            log_files_path: None,
            log_files_format: "%Y-%m-%d".to_string(),
            max_log_files: Some(10),
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

    /// Save logs as files to directory
    pub fn to_files_dir<P>(mut self, path: P) -> Self
    where
        P: Into<std::path::PathBuf>,
    {
        self.log_files_path = Some(path.into());
        self
    }

    /// Keep only N log files removing the old ones.
    /// If none is passed then no files is removed
    pub fn max_log_files(mut self, max: Option<usize>) -> Self {
        self.max_log_files = max;
        self
    }

    /// Format the log files name, by default is "%Y-%m-%d"
    pub fn file_name_format(mut self, format: &str) -> Self {
        self.log_files_format = format.to_string();
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
    eprintln!("Error initializing logs: {e}");
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

#[cfg(not(target_arch = "wasm32"))]
fn chain_save_to_file(dispatch: fern::Dispatch, config: &LogConfig) -> fern::Dispatch {
    use std::io::Write;
    use std::sync::mpsc::channel;

    // if there is no file_path defined just skip
    let Some(path) = &config.log_files_path else {
        return dispatch;
    };

    // create the logs directory if needed
    let res =
        std::fs::create_dir_all(path).map_err(|e| format!("Cannot create save directory: {e}"));
    if let Err(e) = res {
        print_apply_error(&e.to_string());
        return dispatch;
    }

    // remove old logs if necessary
    if let Some(max) = config.max_log_files {
        use std::time::SystemTime;

        // filter by log extensions
        let mut log_files = std::fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|e| {
                        e.path()
                            .extension()
                            .is_some_and(|ext| ext.to_str() == Some(LOG_FILE_EXT))
                            .then_some(e.path())
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // sort by modified time, oldest first
        log_files.sort_by_key(|p| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        });

        // delete old ones
        if log_files.len() > max {
            let remove_len = log_files.len() - max;
            log_files.into_iter().take(remove_len).for_each(|old_file| {
                let _ = std::fs::remove_file(old_file);
            });
        }
    }

    // create or open the file the user requests
    let file_name = chrono::Utc::now()
        .format(&config.log_files_format)
        .to_string();
    let file_path = path.join(file_name).with_extension(LOG_FILE_EXT);

    let file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path);

    let mut file = match file {
        Ok(file) => file,
        Err(e) => {
            print_apply_error(&e.to_string());
            return dispatch;
        }
    };

    // we need a second thread to manage the IO because it's "slow"
    // and in-games we don't want to be waiting for the filesystem alsmot never
    // so we use the sender to send the logs to the other thread
    // WARN: Panic logs may be lost because the second thread could not wait
    // until getting the last message. However in my "manual tests" this seems
    // to be working fine, so I am not overthinking it to find a solution yet.
    let (tx, rx) = channel::<String>();
    std::thread::spawn(move || {
        for ln in rx {
            if let Err(e) = write!(file, "{ln}") {
                eprintln!("Log write line error: {e}")
            }
        }
    });

    dispatch.chain(tx)
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

    let use_colors = config.log_files_path.is_none() && config.colored;
    if use_colors {
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        dispatch = chain_save_to_file(dispatch, &config);
    }

    if let Err(e) = dispatch.apply() {
        print_apply_error(&e.to_string());
    }
}
