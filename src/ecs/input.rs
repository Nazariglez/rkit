use crate::ecs::plugin::Plugin;
use crate::prelude::{App, OnEnginePostFrame, OnEnginePreFrame, OnEngineSetup};
use bevy_ecs::prelude::*;
use corelib::input::{
    hide_cursor, is_cursor_locked, is_cursor_on_screen, is_cursor_visible, is_mouse_moving,
    is_mouse_scrolling, keys_down, keys_pressed, keys_released, lock_cursor, mouse_btns_down,
    mouse_btns_pressed, mouse_btns_released, mouse_motion_delta, mouse_position, mouse_wheel_delta,
    show_cursor, unlock_cursor,
};
use corelib::math::Vec2;

// re-export common use types
pub use corelib::input::{KeyCode, KeyCodeList, MouseButton, MouseButtonList};

// -- Mouse
pub struct MousePlugin;
impl Plugin for MousePlugin {
    fn apply(self, app: App) -> App {
        app.add_systems(OnEngineSetup, init_mouse_system)
            .add_systems(OnEnginePreFrame, populate_mouse_system)
            .add_systems(OnEnginePostFrame, sync_mouse_system)
    }
}

#[derive(Resource)]
pub struct Mouse {
    dirty: bool,
    pos: Vec2,
    motion_delta: Vec2,
    wheel_delta: Vec2,
    btn_down: MouseButtonList,
    btn_pressed: MouseButtonList,
    btn_released: MouseButtonList,
    cursor_lock: bool,
    cursor_visible: bool,
    cursor_on_window: bool,
    moving: bool,
    scrolling: bool,
}

impl Mouse {
    #[inline]
    pub fn position(&self) -> Vec2 {
        self.pos
    }

    #[inline]
    pub fn motion_delta(&self) -> Vec2 {
        self.motion_delta
    }

    #[inline]
    pub fn wheel_delta(&self) -> Vec2 {
        self.wheel_delta
    }

    #[inline]
    pub fn just_pressed(&self, btn: MouseButton) -> bool {
        self.btn_pressed.contains(btn)
    }

    #[inline]
    pub fn just_released(&self, btn: MouseButton) -> bool {
        self.btn_released.contains(btn)
    }

    #[inline]
    pub fn is_down(&self, btn: MouseButton) -> bool {
        self.btn_down.contains(btn)
    }

    #[inline]
    pub fn is_moving(&self) -> bool {
        self.moving
    }

    #[inline]
    pub fn is_scrolling(&self) -> bool {
        self.scrolling
    }

    #[inline]
    pub fn is_cursor_lock(&self) -> bool {
        self.cursor_lock
    }

    #[inline]
    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    #[inline]
    pub fn is_cursor_on_window(&self) -> bool {
        self.cursor_on_window
    }

    #[inline]
    pub fn lock_cursor(&mut self) {
        if self.cursor_lock {
            return;
        }
        self.cursor_lock = true;
        self.dirty = true;
    }

    #[inline]
    pub fn unlock_cursor(&mut self) {
        if !self.cursor_lock {
            return;
        }
        self.cursor_lock = false;
        self.dirty = true;
    }

    #[inline]
    pub fn show_cursor(&mut self) {
        if self.cursor_visible {
            return;
        }
        self.cursor_visible = true;
        self.dirty = true;
    }

    #[inline]
    pub fn hide_cursor(&mut self) {
        if !self.cursor_visible {
            return;
        }
        self.cursor_visible = false;
        self.dirty = true;
    }

    #[inline]
    pub fn down_buttons(&self) -> MouseButtonList {
        self.btn_down.clone()
    }

    #[inline]
    pub fn pressed_buttons(&self) -> MouseButtonList {
        self.btn_pressed.clone()
    }

    #[inline]
    pub fn released_buttons(&self) -> MouseButtonList {
        self.btn_released.clone()
    }

    pub(crate) fn clear_down_btn(&mut self, btn: MouseButton) {
        self.btn_down.remove(btn);
    }

    pub(crate) fn clear_pressed_btn(&mut self, btn: MouseButton) {
        self.btn_pressed.remove(btn);
    }

    pub(crate) fn clear_released_btn(&mut self, btn: MouseButton) {
        self.btn_released.remove(btn);
    }
}

fn init_mouse_system(mut cmds: Commands) {
    cmds.insert_resource(Mouse {
        dirty: false,
        pos: mouse_position(),
        motion_delta: mouse_motion_delta(),
        wheel_delta: mouse_wheel_delta(),
        btn_down: mouse_btns_down(),
        btn_pressed: mouse_btns_pressed(),
        btn_released: mouse_btns_released(),
        cursor_lock: is_cursor_locked(),
        cursor_visible: is_cursor_visible(),
        cursor_on_window: is_cursor_on_screen(),
        moving: is_mouse_moving(),
        scrolling: is_mouse_scrolling(),
    })
}

fn populate_mouse_system(mut mouse: ResMut<Mouse>) {
    mouse.pos = mouse_position();
    mouse.motion_delta = mouse_motion_delta();
    mouse.wheel_delta = mouse_wheel_delta();
    mouse.btn_down = mouse_btns_down();
    mouse.btn_pressed = mouse_btns_pressed();
    mouse.btn_released = mouse_btns_released();
    mouse.cursor_lock = is_cursor_locked();
    mouse.cursor_visible = is_cursor_visible();
    mouse.cursor_on_window = is_cursor_on_screen();
    mouse.moving = is_mouse_moving();
    mouse.scrolling = is_mouse_scrolling();
}

fn sync_mouse_system(mut mouse: ResMut<Mouse>) {
    if !mouse.dirty {
        return;
    }

    mouse.dirty = false;
    let diff_cursor = mouse.cursor_lock != is_cursor_locked();
    if diff_cursor {
        if mouse.cursor_lock {
            lock_cursor();
        } else {
            unlock_cursor();
        }
    }

    let diff_visibility = mouse.cursor_visible != is_cursor_visible();
    if diff_visibility {
        if mouse.cursor_visible {
            hide_cursor();
        } else {
            show_cursor();
        }
    }
}

// -- Keyboard
pub struct KeyboardPlugin;
impl Plugin for KeyboardPlugin {
    fn apply(self, app: App) -> App {
        app.add_systems(OnEngineSetup, init_keyboard_system)
            .add_systems(OnEnginePreFrame, populate_keyboard_system)
    }
}

#[derive(Resource)]
pub struct Keyboard {
    key_pressed: KeyCodeList,
    key_down: KeyCodeList,
    key_released: KeyCodeList,
}

impl Keyboard {
    pub fn is_down(&self, key: KeyCode) -> bool {
        self.key_down.contains(key)
    }

    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.key_pressed.contains(key)
    }

    pub fn just_released(&self, key: KeyCode) -> bool {
        self.key_released.contains(key)
    }

    #[inline]
    pub fn down_keys(&self) -> KeyCodeList {
        self.key_down.clone()
    }

    #[inline]
    pub fn pressed_keys(&self) -> KeyCodeList {
        self.key_pressed.clone()
    }

    #[inline]
    pub fn released_keys(&self) -> KeyCodeList {
        self.key_released.clone()
    }
}

fn init_keyboard_system(mut cmds: Commands) {
    cmds.insert_resource(Keyboard {
        key_pressed: keys_pressed(),
        key_down: keys_down(),
        key_released: keys_released(),
    })
}

fn populate_keyboard_system(mut keyboard: ResMut<Keyboard>) {
    keyboard.key_down = keys_down();
    keyboard.key_pressed = keys_pressed();
    keyboard.key_released = keys_released();
}
