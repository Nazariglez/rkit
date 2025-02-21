use super::app::App;
use super::plugin::Plugin;
use crate::app::LogConfig;
use crate::macros::Deref;
use bevy_ecs::prelude::*;

#[derive(Resource, Deref, Default)]
pub struct LogPlugin(LogConfig);

impl Plugin for LogPlugin {
    fn apply(self, app: &mut App) -> &mut App {
        app.with_log(self.0)
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
}
