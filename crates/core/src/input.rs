use crate::backend::{get_backend, BackendImpl};
use math::Vec2;

// Mouse
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[repr(u8)]
pub enum MouseButton {
    Left = 0b00000001,
    Middle = 0b00000010,
    Right = 0b00000100,
    Back = 0b00001000,
    Forward = 0b00010000,

    // Unknown is the last value
    Unknown = 0b10000000,
}

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct MouseState {
    pub position: Vec2,
    pub motion_delta: Vec2,
    pub wheel_delta: Vec2,
    pub moving: bool,
    pub scrolling: bool,

    pressed: u8,
    released: u8,
    down: u8,
}

impl MouseState {
    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn motion_delta(&self) -> Vec2 {
        self.motion_delta
    }

    pub fn wheel_delta(&self) -> Vec2 {
        self.wheel_delta
    }

    pub fn is_moving(&self) -> bool {
        self.moving
    }

    pub fn is_scrolling(&self) -> bool {
        self.scrolling
    }

    pub fn press(&mut self, btn: MouseButton) {
        let value = btn as u8;
        self.pressed |= value;
        self.down |= value;
        self.released &= !value;
    }

    pub fn release(&mut self, btn: MouseButton) {
        let value = btn as u8;
        self.released |= value;
        self.down &= !value;
        self.pressed &= !value;
    }

    pub fn are_pressed<const N: usize>(&self, btns: &[MouseButton; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_pressed(btn);
        });
        res
    }

    pub fn is_pressed(&self, btn: MouseButton) -> bool {
        (self.pressed & (btn as u8)) != 0
    }

    pub fn are_released<const N: usize>(&self, btns: &[MouseButton; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_released(btn);
        });
        res
    }

    pub fn is_released(&self, btn: MouseButton) -> bool {
        (self.released & (btn as u8)) != 0
    }

    pub fn are_down<const N: usize>(&self, btns: &[MouseButton; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_down(btn);
        });
        res
    }

    pub fn is_down(&self, btn: MouseButton) -> bool {
        (self.down & (btn as u8)) != 0
    }

    pub fn tick(&mut self) {
        self.pressed = 0;
        self.released = 0;
        self.motion_delta = Vec2::ZERO;
        self.moving = false;
        self.wheel_delta = Vec2::ZERO;
        self.scrolling = false;
    }
}

// Keyboard

// -- Input section
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
pub fn is_mouse_btn_released(btn: MouseButton) -> bool {
    get_backend().mouse_state().is_released(btn)
}

#[inline]
pub fn are_mouse_btns_released<const N: usize>(btns: &[MouseButton; N]) -> [bool; N] {
    get_backend().mouse_state().are_released(btns)
}

#[inline]
pub fn is_mouse_btn_down(btn: MouseButton) -> bool {
    get_backend().mouse_state().is_down(btn)
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
