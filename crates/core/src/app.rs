use crate::backend::{get_backend, get_mut_backend, BackendImpl};
use math::{vec2, Vec2};

// -- Window section
#[inline]
pub fn window_title() -> String {
    get_backend().title()
}

#[inline]
pub fn set_window_title(title: &str) {
    get_mut_backend().set_title(title)
}

#[inline]
pub fn window_size() -> Vec2 {
    get_backend().size()
}

#[inline]
pub fn set_window_size(width: f32, height: f32) {
    get_mut_backend().set_size(vec2(width, height));
}

#[inline]
pub fn window_width() -> f32 {
    get_backend().size().x
}

#[inline]
pub fn window_height() -> f32 {
    get_backend().size().y
}

#[inline]
pub fn is_window_fullscreen() -> bool {
    get_backend().is_fullscreen()
}

#[inline]
pub fn toggle_fullscreen() {
    get_mut_backend().toggle_fullscreen()
}

#[inline]
pub fn window_dpi_scale() -> f32 {
    get_backend().dpi()
}

#[inline]
pub fn window_position() -> Vec2 {
    get_backend().position()
}

#[inline]
pub fn set_window_position(x: f32, y: f32) {
    get_mut_backend().set_position(x, y);
}

#[inline]
pub fn is_window_focused() -> bool {
    get_backend().is_focused()
}

#[inline]
pub fn is_window_maximized() -> bool {
    get_backend().is_maximized()
}

#[inline]
pub fn is_window_minimized() -> bool {
    get_backend().is_minimized()
}

#[inline]
pub fn screen_size() -> Vec2 {
    get_backend().screen_size()
}
