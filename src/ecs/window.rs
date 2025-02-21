use crate::prelude::{App, OnEnginePostFrame, OnEnginePreFrame, OnEngineSetup, Plugin};
use bevy_ecs::prelude::*;
use corelib::app::*;
use corelib::math::{uvec2, Vec2};
use macros::Deref;

#[derive(Default)]
pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn apply(&self, app: &mut App) {
        app.add_systems(OnEngineSetup, init_window_system)
            .add_systems(OnEnginePreFrame, populate_window_system)
            .add_systems(OnEnginePostFrame, sync_window_system);
    }
}

#[derive(Default, Debug, Resource, Deref)]
pub struct WindowConfigPlugin(WindowConfig);

impl Plugin for WindowConfigPlugin {
    fn apply(&self, app: &mut App) {
        app.with_window(self.0.clone());
    }
}

impl WindowConfigPlugin {
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

    /// Open the window maximized
    /// `Web`: Will use the parent's size
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
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

#[derive(Resource, Default)]
pub struct Window {
    dirty: bool,
    size: Vec2,
    title: String,
    min_size: Option<Vec2>,
    max_size: Option<Vec2>,
    fullscreen: bool,
    dpi_scale: f32,
    position: Vec2,
    focused: bool,
    maximized: bool,
    minimized: bool,
    screen_size: Vec2,
    pixelated: bool,
    close_request: bool,
}

impl Window {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: &str) {
        if title == self.title.as_str() {
            return;
        }

        self.title = title.to_string();
        self.dirty = true;
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn set_size(&mut self, size: Vec2) {
        if self.size == size {
            return;
        }

        self.size = size;
        self.dirty = true;
    }

    pub fn set_min_size(&mut self, size: Vec2) {
        if self.min_size == Some(size) {
            return;
        }

        self.min_size = Some(size);
        self.dirty = true;
    }

    pub fn set_max_size(&mut self, size: Vec2) {
        if self.max_size == Some(size) {
            return;
        }

        self.max_size = Some(size);
        self.dirty = true;
    }

    pub fn width(&self) -> f32 {
        self.size.x
    }

    pub fn height(&self) -> f32 {
        self.size.y
    }

    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        if self.fullscreen == fullscreen {
            return;
        }

        self.fullscreen = fullscreen;
        self.dirty = true;
    }

    pub fn dpi_scale(&self) -> f32 {
        self.dpi_scale
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn set_position(&mut self, position: Vec2) {
        if self.position == position {
            return;
        }

        self.position = position;
        self.dirty = true;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn is_maximized(&self) -> bool {
        self.maximized
    }

    pub fn is_minimized(&self) -> bool {
        self.minimized
    }

    pub fn is_pixelated(&self) -> bool {
        self.pixelated
    }

    pub fn screen_size(&self) -> Vec2 {
        self.screen_size
    }

    pub fn close(&mut self) {
        self.close_request = true;
        self.dirty = true;
    }
}

fn init_window_system(mut cmds: Commands) {
    cmds.insert_resource(Window {
        dirty: false,
        size: window_size(),
        title: window_title(),
        min_size: None,
        max_size: None,
        fullscreen: is_window_fullscreen(),
        dpi_scale: window_dpi_scale(),
        position: window_position(),
        focused: is_window_focused(),
        maximized: is_window_maximized(),
        minimized: is_window_minimized(),
        screen_size: screen_size(),
        pixelated: is_window_pixelated(),
        close_request: false,
    });
}

fn populate_window_system(mut win: ResMut<Window>) {
    // TODO: add on notan/rkit a window_dirty() flag?
    win.size = window_size();
    win.fullscreen = is_window_fullscreen();
    win.focused = is_window_focused();
    win.dpi_scale = window_dpi_scale();
    win.screen_size = screen_size();
    win.position = window_position();
    win.minimized = is_window_minimized();
    win.maximized = is_window_maximized();
}

fn sync_window_system(mut win: ResMut<Window>) {
    if !win.dirty {
        return;
    }

    win.dirty = false;

    set_window_size(win.size.x, win.size.y);
    let fullscreen = is_window_fullscreen();
    if fullscreen != is_window_fullscreen() {
        toggle_fullscreen();
    }
    set_window_title(&win.title);
    set_window_position(win.position.x, win.position.y);
    if let Some(size) = win.min_size {
        set_window_min_size(size.x, size.y);
    }
    if let Some(size) = win.max_size {
        set_window_max_size(size.x, size.y);
    }

    if win.close_request {
        close_window();
    }
}
