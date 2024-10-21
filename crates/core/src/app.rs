use crate::backend::{get_backend, get_mut_backend, BackendImpl};
use crate::math::{vec2, Vec2};

mod window;
pub use window::*;

#[cfg(feature = "logs")]
pub mod logger;
#[cfg(feature = "logs")]
pub use logger::LogConfig;

use crate::events::{CoreEvent, CORE_EVENTS_MAP};

// -- Window section

/// Return the window's title
#[inline]
pub fn window_title() -> String {
    get_backend().title()
}

/// Set the window's title
/// `Web`: Does nothing
#[inline]
pub fn set_window_title(title: &str) {
    get_mut_backend().set_title(title)
}

/// Returns the size of the window
/// `Web`: Returns the size of the canvas
#[inline]
pub fn window_size() -> Vec2 {
    get_backend().size()
}

/// Set the window's size
#[inline]
pub fn set_window_size(width: f32, height: f32) {
    get_mut_backend().set_size(vec2(width, height));
}

/// Set the minimum size the window can be
#[inline]
pub fn set_window_min_size(width: f32, height: f32) {
    get_mut_backend().set_min_size(vec2(width, height));
}

/// Set the maximum size the window can be
#[inline]
pub fn set_window_max_size(width: f32, height: f32) {
    get_mut_backend().set_max_size(vec2(width, height));
}

/// Return the window's width
#[inline]
pub fn window_width() -> f32 {
    get_backend().size().x
}

/// Return the window's height
#[inline]
pub fn window_height() -> f32 {
    get_backend().size().y
}

/// Return if the windows is currently in fullscreen mode
#[inline]
pub fn is_window_fullscreen() -> bool {
    get_backend().is_fullscreen()
}

/// Swap between fullscreen/windowed mode
#[inline]
pub fn toggle_fullscreen() {
    get_mut_backend().toggle_fullscreen()
}

/// Return window's resolution (based on the screen dpi)
#[inline]
pub fn window_dpi_scale() -> f32 {
    get_backend().dpi()
}

/// Return the current window's position
#[inline]
pub fn window_position() -> Vec2 {
    get_backend().position()
}

/// Set the window's position
#[inline]
pub fn set_window_position(x: f32, y: f32) {
    get_mut_backend().set_position(x, y);
}

/// Returns if the window is right now focused
#[inline]
pub fn is_window_focused() -> bool {
    get_backend().is_focused()
}

/// Return if the window is maximized
#[inline]
pub fn is_window_maximized() -> bool {
    get_backend().is_maximized()
}

/// Return if the window is minimized
#[inline]
pub fn is_window_minimized() -> bool {
    get_backend().is_minimized()
}

/// Return if the window offset texture is using nearest filter
#[inline]
pub fn is_window_pixelated() -> bool {
    get_backend().is_pixelated()
}

/// Close the window
/// `Web`: Stops the `requestAnimationFrame`
#[inline]
pub fn close_window() {
    get_mut_backend().close()
}

/// Return the screen's size
/// `Web`: Returns the canvas parent's size
#[inline]
pub fn screen_size() -> Vec2 {
    get_backend().screen_size()
}

// - Core events
/// Callback executed after the init.
/// This is primarily intended for use by plugins, not by the end user.
#[inline]
pub fn on_sys_init<F: Fn() + Send + Sync + 'static>(cb: F) {
    CORE_EVENTS_MAP.borrow_mut().insert(CoreEvent::Init, cb);
}

/// Callback executed before the update callback.
/// This is primarily intended for use by plugins, not by the end user.
#[inline]
pub fn on_sys_pre_update<F: Fn() + Send + Sync + 'static>(cb: F) {
    CORE_EVENTS_MAP
        .borrow_mut()
        .insert(CoreEvent::PreUpdate, cb);
}

/// Callback executed after the update callback.
/// This is primarily intended for use by plugins, not by the end user.
#[inline]
pub fn on_sys_post_update<F: Fn() + Send + Sync + 'static>(cb: F) {
    CORE_EVENTS_MAP
        .borrow_mut()
        .insert(CoreEvent::PostUpdate, cb);
}

/// Callback executed after the cleanup callback.
/// This is primarily intended for use by plugins, not by the end user.
#[inline]
pub fn on_sys_cleanup<F: Fn() + Send + Sync + 'static>(cb: F) {
    CORE_EVENTS_MAP.borrow_mut().insert(CoreEvent::CleanUp, cb);
}
