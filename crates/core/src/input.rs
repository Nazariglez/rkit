use crate::backend::{get_backend, BackendImpl};
use crate::math::Vec2;

mod gamepad;
mod keyboard;
mod mouse;
mod touch;

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
#[inline]
pub fn gamepads_available() -> GamepadList {
    get_backend().gamepad_state().available()
}

#[inline]
pub fn is_gamepad_btn_pressed(id: GamepadId, btn: GamepadButton) -> bool {
    get_backend()
        .gamepad_state()
        .get(id.raw())
        .map(|info| info.is_pressed(btn))
        .unwrap_or_default()
}

#[inline]
pub fn are_gamepad_btns_pressed<const N: usize>(
    id: GamepadId,
    btns: &[GamepadButton; N],
) -> [bool; N] {
    get_backend()
        .gamepad_state()
        .get(id.raw())
        .map(|info| info.are_pressed(btns))
        .unwrap_or([false; N])
}

#[inline]
pub fn gamepad_btns_pressed(id: GamepadId) -> GamepadButtonList {
    get_backend()
        .gamepad_state()
        .get(id.raw())
        .map(|info| info.pressed.clone())
        .unwrap_or_default()
}

#[inline]
pub fn is_gamepad_btn_released(id: GamepadId, btn: GamepadButton) -> bool {
    get_backend()
        .gamepad_state()
        .get(id.raw())
        .map(|info| info.is_released(btn))
        .unwrap_or_default()
}

#[inline]
pub fn are_gamepad_btns_released<const N: usize>(
    id: GamepadId,
    btns: &[GamepadButton; N],
) -> [bool; N] {
    get_backend()
        .gamepad_state()
        .get(id.raw())
        .map(|info| info.are_released(btns))
        .unwrap_or([false; N])
}

#[inline]
pub fn gamepad_btns_released(id: GamepadId) -> GamepadButtonList {
    get_backend()
        .gamepad_state()
        .get(id.raw())
        .map(|info| info.released.clone())
        .unwrap_or_default()
}

#[inline]
pub fn is_gamepad_btn_down(id: GamepadId, btn: GamepadButton) -> bool {
    get_backend()
        .gamepad_state()
        .get(id.raw())
        .map(|info| info.is_down(btn))
        .unwrap_or_default()
}

#[inline]
pub fn gamepad_btns_down(id: GamepadId) -> GamepadButtonList {
    get_backend()
        .gamepad_state()
        .get(id.raw())
        .map(|info| info.down.clone())
        .unwrap_or_default()
}

#[inline]
pub fn are_gamepad_btns_down<const N: usize>(
    id: GamepadId,
    btns: &[GamepadButton; N],
) -> [bool; N] {
    get_backend()
        .gamepad_state()
        .get(id.raw())
        .map(|info| info.are_down(btns))
        .unwrap_or([false; N])
}

#[inline]
pub fn gamepad_axis_movement(id: GamepadId, axis: GamepadAxis) -> f32 {
    get_backend()
        .gamepad_state()
        .get(id.raw())
        .map(|info| info.axis_strength(axis))
        .unwrap_or_default()
}

// TODO gamepad name?
// #[inline]
// pub fn gamepad_name<'a>(id: GamepadId) -> &'a str {
//     todo!()
// }

// -- Touch/Gestures
// TODO touch/gestures
