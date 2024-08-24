use crate::backend::{get_backend, BackendImpl};
use math::Vec2;

mod gamepad;
mod keyboard;
mod mouse;

pub use gamepad::*;
pub use keyboard::*;
pub use mouse::*;

// -- Mouse
#[inline]
pub fn mouse_position() -> Vec2 {
    get_backend().mouse_state().position()
}

#[inline]
pub fn mouse_motion_delta() -> Vec2 {
    get_backend().mouse_state().motion_delta()
}

#[inline]
pub fn mouse_wheel_delta() -> Vec2 {
    get_backend().mouse_state().wheel_delta()
}

#[inline]
pub fn is_mouse_btn_pressed(btn: MouseButton) -> bool {
    get_backend().mouse_state().is_pressed(btn)
}

#[inline]
pub fn are_mouse_btns_pressed<const N: usize>(btns: &[MouseButton; N]) -> [bool; N] {
    get_backend().mouse_state().are_pressed(btns)
}

#[inline]
pub fn mouse_btns_pressed() -> MouseButtonList {
    get_backend().mouse_state().pressed.clone()
}

#[inline]
pub fn is_mouse_btn_released(btn: MouseButton) -> bool {
    get_backend().mouse_state().is_released(btn)
}

#[inline]
pub fn are_mouse_btns_released<const N: usize>(btns: &[MouseButton; N]) -> [bool; N] {
    get_backend().mouse_state().are_released(btns)
}

#[inline]
pub fn mouse_btns_released() -> MouseButtonList {
    get_backend().mouse_state().released.clone()
}

#[inline]
pub fn is_mouse_btn_down(btn: MouseButton) -> bool {
    get_backend().mouse_state().is_down(btn)
}

#[inline]
pub fn mouse_btns_down() -> MouseButtonList {
    get_backend().mouse_state().down.clone()
}

#[inline]
pub fn are_mouse_btns_down<const N: usize>(btns: &[MouseButton; N]) -> [bool; N] {
    get_backend().mouse_state().are_down(btns)
}

#[inline]
pub fn is_mouse_moving() -> bool {
    get_backend().mouse_state().is_moving()
}

#[inline]
pub fn is_mouse_scrolling() -> bool {
    get_backend().mouse_state().is_scrolling()
}

// -- Keyboard
#[inline]
pub fn is_key_pressed(key: KeyCode) -> bool {
    get_backend().keyboard_state().is_pressed(key)
}

#[inline]
pub fn are_keys_pressed<const N: usize>(keys: &[KeyCode; N]) -> [bool; N] {
    get_backend().keyboard_state().are_pressed(keys)
}

#[inline]
pub fn keys_pressed() -> KeyCodeList {
    get_backend().keyboard_state().pressed.clone()
}

#[inline]
pub fn is_key_released(key: KeyCode) -> bool {
    get_backend().keyboard_state().is_released(key)
}

#[inline]
pub fn are_keys_released<const N: usize>(keys: &[KeyCode; N]) -> [bool; N] {
    get_backend().keyboard_state().are_released(keys)
}

#[inline]
pub fn keys_released() -> KeyCodeList {
    get_backend().keyboard_state().released.clone()
}

#[inline]
pub fn is_key_down(key: KeyCode) -> bool {
    get_backend().keyboard_state().is_down(key)
}

#[inline]
pub fn are_keys_down<const N: usize>(keys: &[KeyCode; N]) -> [bool; N] {
    get_backend().keyboard_state().are_down(keys)
}

#[inline]
pub fn keys_down() -> KeyCodeList {
    get_backend().keyboard_state().down.clone()
}

#[inline]
pub fn text_pressed() -> TextList {
    get_backend().keyboard_state().text.clone()
}

// -- Gamepad
// TODO gamepad API