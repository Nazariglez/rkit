use super::app::App;
use super::plugin::Plugin;
use crate::app::LogConfig;
use crate::macros::Deref;
use bevy_ecs::prelude::*;

#[derive(Resource, Deref, Default)]
pub struct LogPlugin(LogConfig);

impl Plugin for LogPlugin {
    fn apply(&self, app: &mut App) {
        app.add_log(self.0.clone());
    }
}

impl LogPlugin {
    /// Creates a new configuration using the given level filter
    pub fn new(level: log::LevelFilter) -> Self {
        Self(LogConfig::new(level))
    }

    /// Configure logs to use trace level filter
    pub fn trace() -> Self {
        Self(LogConfig::trace())
    }

    /// Configure logs to use debug level filter
    pub fn debug() -> Self {
        Self(LogConfig::debug())
    }

    /// Configure logs to use info level filter
    pub fn info() -> Self {
        Self(LogConfig::info())
    }

    /// Configure logs to use warn level filter
    pub fn warn() -> Self {
        Self(LogConfig::warn())
    }

    /// Configure logs to use error level filter
    pub fn error() -> Self {
        Self(LogConfig::error())
    }

    /// Changes the level filter
    pub fn level(mut self, level: log::LevelFilter) -> Self {
        self.0 = self.0.level(level);
        self
    }

    /// Change the filter level for dependencies
    pub fn level_for(mut self, id: &str, level: log::LevelFilter) -> Self {
        self.0 = self.0.level_for(id, level);
        self
    }

    /// Enable colored text (Defaults to true on debug mode)
    pub fn use_colors(mut self, value: bool) -> Self {
        self.0 = self.0.use_colors(value);
        self
    }

    /// Log everything including dependencies
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.0 = self.0.verbose(verbose);
        self
    }

    /// Log files to a directory
    pub fn to_files_dir<P>(mut self, path: P) -> Self
    where
        P: Into<std::path::PathBuf>,
    {
        self.0 = self.0.to_files_dir(path);
        self
    }

    /// Keep only N log files removing the old ones.
    /// If none is passed then no files is removed
    pub fn max_log_files(mut self, max: Option<usize>) -> Self {
        self.0 = self.0.max_log_files(max);
        self
    }

    /// Format the log files name, by default is "%Y-%m-%d"
    pub fn file_name_format(mut self, format: &str) -> Self {
        self.0 = self.0.file_name_format(format);
        self
    }
}
