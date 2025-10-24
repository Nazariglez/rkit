use crate::math::{UVec2, uvec2};

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub size: UVec2,
    pub min_size: Option<UVec2>,
    pub max_size: Option<UVec2>,
    pub resizable: bool,
    pub maximized: bool,
    pub vsync: bool,
    pub max_fps: Option<u8>,
    pub pixelated: bool,
    pub cursor: bool,
    pub fullscreen: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "GameKit Window".to_string(),
            size: uvec2(800, 600),
            min_size: None,
            max_size: None,
            resizable: true,
            maximized: false,
            vsync: true,
            max_fps: None,
            pixelated: false,
            cursor: true,
            fullscreen: false,
        }
    }
}

impl WindowConfig {
    /// Set the window's title
    #[inline]
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Set the window's size
    #[inline]
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = uvec2(width, height);
        self
    }

    /// Set the window's maximum size
    #[inline]
    pub fn max_size(mut self, width: u32, height: u32) -> Self {
        self.max_size = Some(uvec2(width, height));
        self
    }

    /// Set the window's minimum size
    #[inline]
    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some(uvec2(width, height));
        self
    }

    /// Allow the window to be resizable
    #[inline]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Open the window maximized
    /// `Web`: Will use the parent's size
    #[inline]
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// Enables Vertical Synchronization
    #[inline]
    pub fn vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    /// Limits the maximum fps
    #[inline]
    pub fn max_fps(mut self, fps: u8) -> Self {
        self.max_fps = Some(fps);
        self
    }

    /// Use Nearest filter for the offscreen texture
    #[inline]
    pub fn pixelated(mut self, pixelated: bool) -> Self {
        self.pixelated = pixelated;
        self
    }

    /// Hide or show the cursor
    #[inline]
    pub fn cursor(mut self, visible: bool) -> Self {
        self.cursor = visible;
        self
    }
}
