use crate::input::{GamepadState, KeyboardState, MouseState};
use crate::math::Vec2;

pub(crate) trait BackendImpl {
    // Window
    fn set_title(&mut self, title: &str);
    fn title(&self) -> String;
    fn size(&self) -> Vec2;
    fn set_size(&mut self, size: Vec2);
    fn set_min_size(&mut self, size: Vec2);
    fn set_max_size(&mut self, size: Vec2);
    fn screen_size(&self) -> Vec2;
    fn is_fullscreen(&self) -> bool;
    fn toggle_fullscreen(&mut self);
    fn dpi(&self) -> f32;
    fn position(&self) -> Vec2;
    fn set_position(&mut self, x: f32, y: f32);
    fn is_focused(&self) -> bool;
    fn is_maximized(&self) -> bool;
    fn is_minimized(&self) -> bool;
    fn close(&mut self);

    // input
    fn mouse_state(&self) -> &MouseState;
    fn keyboard_state(&self) -> &KeyboardState;
    fn gamepad_state(&self) -> &GamepadState;
}
