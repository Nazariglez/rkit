use crate::math::{uvec2, UVec2};

#[derive(Debug)]
pub struct WindowConfig {
    pub title: String,
    pub size: UVec2,
    pub min_size: Option<UVec2>,
    pub max_size: Option<UVec2>,
    pub resizable: bool,
    pub vsync: bool,
    pub max_fps: Option<u8>,
    pub pixelated: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "GameKit Window".to_string(),
            size: uvec2(800, 600),
            min_size: None,
            max_size: None,
            resizable: true,
            vsync: true,
            max_fps: None,
            pixelated: false,
        }
    }
}

impl WindowConfig {
    /// Set the window's title
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Set the window's size
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = uvec2(width, height);
        self
    }

    /// Set the window's maximum size
    pub fn max_size(mut self, width: u32, height: u32) -> Self {
        self.max_size = Some(uvec2(width, height));
        self
    }

    /// Set the window's minimum size
    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some(uvec2(width, height));
        self
    }

    /// Allow the window to be resizable
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Enables Vertical Synchronization
    pub fn vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    /// Limits the maximum fps
    pub fn max_fps(mut self, fps: u8) -> Self {
        self.max_fps = Some(fps);
        self
    }

    /// Use Nearest filter for the offscreen texture
    pub fn pixelated(mut self, pixelated: bool) -> Self {
        self.pixelated = pixelated;
        self
    }
}