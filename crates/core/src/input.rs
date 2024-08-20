use crate::backend::{get_backend, BackendImpl};
use math::Vec2;

// TODO wheel, etc...

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
    pressed: u8,
    released: u8,
    down: u8,
}

impl MouseState {
    pub fn position(&self) -> Vec2 {
        self.position
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

    pub fn is_pressed(&self, btn: MouseButton) -> bool {
        (self.pressed & (btn as u8)) != 0
    }

    pub fn is_released(&self, btn: MouseButton) -> bool {
        (self.released & (btn as u8)) != 0
    }

    pub fn is_down(&self, btn: MouseButton) -> bool {
        (self.down & (btn as u8)) != 0
    }

    pub fn tick(&mut self) {
        self.pressed = 0;
        self.released = 0;
    }

    pub fn clear(&mut self) {
        self.pressed = 0;
        self.down = 0;
        self.released = 0;
    }
}

// -- Input section
#[inline]
pub fn mouse_position() -> Vec2 {
    get_backend().mouse_state().position()
}
#[inline]
pub fn is_mouse_btn_pressed(btn: MouseButton) -> bool {
    get_backend().mouse_state().is_pressed(btn)
}

#[inline]
pub fn is_mouse_btn_released(btn: MouseButton) -> bool {
    get_backend().mouse_state().is_released(btn)
}

#[inline]
pub fn is_mouse_btn_down(btn: MouseButton) -> bool {
    get_backend().mouse_state().is_down(btn)
}
