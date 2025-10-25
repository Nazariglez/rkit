use crate::ecs::plugin::Plugin;
use crate::prelude::{App, OnEnginePostFrame, OnEnginePreFrame, OnEngineSetup};
use bevy_ecs::prelude::*;
use corelib::{
    input::{
        hide_cursor, is_cursor_locked, is_cursor_on_screen, is_cursor_visible, is_mouse_moving,
        is_mouse_scrolling, keys_down, keys_pressed, keys_released, lock_cursor, mouse_btns_down,
        mouse_btns_pressed, mouse_btns_released, mouse_motion_delta, mouse_position,
        mouse_wheel_delta, show_cursor, text_pressed, unlock_cursor,
    },
    math::Vec2,
};

// re-export common use types
pub use corelib::input::{KeyCode, KeyCodeList, MouseButton, MouseButtonList, TextList};

#[derive(SystemSet, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct InputSysSet;

// -- Mouse
pub struct MousePlugin;
impl Plugin for MousePlugin {
    fn apply(&self, app: &mut App) {
        app.on_schedule(OnEngineSetup, init_mouse_system.in_set(InputSysSet))
            .on_schedule(OnEnginePreFrame, populate_mouse_system.in_set(InputSysSet))
            .on_schedule(OnEnginePostFrame, sync_mouse_system.in_set(InputSysSet))
            .configure_sets(OnEngineSetup, InputSysSet)
            .configure_sets(OnEnginePreFrame, InputSysSet)
            .configure_sets(OnEnginePostFrame, InputSysSet);
    }
}

/// Mouse input state
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
    just_enter: bool,
    just_left: bool,
}

impl Mouse {
    /// Current cursor position in screen coordinates
    #[inline]
    pub fn position(&self) -> Vec2 {
        self.pos
    }

    /// Mouse movement since last frame
    #[inline]
    pub fn motion_delta(&self) -> Vec2 {
        self.motion_delta
    }

    /// Scroll wheel movement since last frame
    #[inline]
    pub fn wheel_delta(&self) -> Vec2 {
        self.wheel_delta
    }

    /// Returns true if button was pressed this frame
    #[inline]
    pub fn just_pressed(&self, btn: MouseButton) -> bool {
        self.btn_pressed.contains(btn)
    }

    /// Returns true if button was released this frame
    #[inline]
    pub fn just_released(&self, btn: MouseButton) -> bool {
        self.btn_released.contains(btn)
    }

    /// Returns true if button is currently held
    #[inline]
    pub fn is_down(&self, btn: MouseButton) -> bool {
        self.btn_down.contains(btn)
    }

    /// Returns true if mouse moved this frame
    #[inline]
    pub fn is_moving(&self) -> bool {
        self.moving
    }

    /// Returns true if scroll wheel was used this frame
    #[inline]
    pub fn is_scrolling(&self) -> bool {
        self.scrolling
    }

    /// Returns true if cursor is locked to window
    #[inline]
    pub fn is_cursor_lock(&self) -> bool {
        self.cursor_lock
    }

    /// Returns true if cursor is visible
    #[inline]
    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Returns true if cursor is on the window
    #[inline]
    pub fn is_cursor_on_window(&self) -> bool {
        self.cursor_on_window
    }

    /// Locks cursor to window
    #[inline]
    pub fn lock_cursor(&mut self) {
        if self.cursor_lock {
            return;
        }
        self.cursor_lock = true;
        self.dirty = true;
    }

    /// Unlocks cursor from window
    #[inline]
    pub fn unlock_cursor(&mut self) {
        if !self.cursor_lock {
            return;
        }
        self.cursor_lock = false;
        self.dirty = true;
    }

    /// Makes cursor visible
    #[inline]
    pub fn show_cursor(&mut self) {
        if self.cursor_visible {
            return;
        }
        self.cursor_visible = true;
        self.dirty = true;
    }

    /// Hides cursor
    #[inline]
    pub fn hide_cursor(&mut self) {
        if !self.cursor_visible {
            return;
        }
        self.cursor_visible = false;
        self.dirty = true;
    }

    /// Returns all currently held buttons
    #[inline]
    pub fn down_buttons(&self) -> MouseButtonList {
        self.btn_down.clone()
    }

    /// Returns all buttons pressed this frame
    #[inline]
    pub fn pressed_buttons(&self) -> MouseButtonList {
        self.btn_pressed.clone()
    }

    /// Returns all buttons released this frame
    #[inline]
    pub fn released_buttons(&self) -> MouseButtonList {
        self.btn_released.clone()
    }

    /// Returns true if cursor entered window this frame
    #[inline]
    pub fn just_enter(&self) -> bool {
        self.just_enter
    }

    /// Returns true if cursor left window this frame
    #[inline]
    pub fn just_left(&self) -> bool {
        self.just_left
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

    /// Allow mutating the mouse state from outside the system
    /// This is useful for systems like gamepad translation to mouse.
    #[inline]
    pub fn mut_handle(&mut self) -> MouseMutHandle<'_> {
        MouseMutHandle { mouse: self }
    }
}

/// Provides mutable access to mouse state for manual control
pub struct MouseMutHandle<'a> {
    mouse: &'a mut Mouse,
}

impl<'a> MouseMutHandle<'a> {
    /// Updates the cursor position
    #[inline]
    pub fn set_position(&mut self, pos: Vec2) {
        self.mouse.pos = pos;
    }

    /// Simulates mouse movement since last frame
    #[inline]
    pub fn set_motion_delta(&mut self, delta: Vec2) {
        self.mouse.motion_delta = delta;
    }

    /// Marks a button as currently held
    #[inline]
    pub fn set_btn_down(&mut self, btn: MouseButton) {
        self.mouse.btn_down.insert(btn);
    }

    /// Marks a button as not held
    #[inline]
    pub fn clear_btn_down(&mut self, btn: MouseButton) {
        self.mouse.btn_down.remove(btn);
    }

    /// Registers a button press event for this frame
    #[inline]
    pub fn set_btn_pressed(&mut self, btn: MouseButton) {
        self.mouse.btn_pressed.insert(btn);
    }

    /// Clears a button press event
    #[inline]
    pub fn clear_btn_pressed(&mut self, btn: MouseButton) {
        self.mouse.btn_pressed.remove(btn);
    }

    /// Registers a button release event for this frame
    #[inline]
    pub fn set_btn_released(&mut self, btn: MouseButton) {
        self.mouse.btn_released.insert(btn);
    }

    /// Clears a button release event
    #[inline]
    pub fn clear_btn_released(&mut self, btn: MouseButton) {
        self.mouse.btn_released.remove(btn);
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
        just_enter: false,
        just_left: false,
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
    mouse.moving = is_mouse_moving();
    mouse.scrolling = is_mouse_scrolling();

    let was_on_window = mouse.cursor_on_window;
    let is_on_window = is_cursor_on_screen();
    mouse.just_left = was_on_window && !is_on_window;
    mouse.just_enter = is_on_window && !was_on_window;
    mouse.cursor_on_window = is_on_window;
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
    fn apply(&self, app: &mut App) {
        app.on_schedule(OnEngineSetup, init_keyboard_system.in_set(InputSysSet))
            .on_schedule(
                OnEnginePreFrame,
                populate_keyboard_system.in_set(InputSysSet),
            )
            .configure_sets(OnEngineSetup, InputSysSet)
            .configure_sets(OnEnginePreFrame, InputSysSet);
    }
}

/// Keyboard input state
#[derive(Resource)]
pub struct Keyboard {
    key_pressed: KeyCodeList,
    key_down: KeyCodeList,
    key_released: KeyCodeList,
    text_pressed: TextList,
}

impl Keyboard {
    /// Returns true if key is currently held
    #[inline]
    pub fn is_down(&self, key: KeyCode) -> bool {
        self.key_down.contains(key)
    }

    /// Returns true if key was pressed this frame
    #[inline]
    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.key_pressed.contains(key)
    }

    /// Returns true if key was released this frame
    #[inline]
    pub fn just_released(&self, key: KeyCode) -> bool {
        self.key_released.contains(key)
    }

    /// Returns all currently held keys
    #[inline]
    pub fn down_keys(&self) -> KeyCodeList {
        self.key_down.clone()
    }

    /// Returns all keys pressed this frame
    #[inline]
    pub fn pressed_keys(&self) -> KeyCodeList {
        self.key_pressed.clone()
    }

    /// Returns all keys released this frame
    #[inline]
    pub fn released_keys(&self) -> KeyCodeList {
        self.key_released.clone()
    }

    /// Returns all text input received this frame
    #[inline]
    pub fn pressed_text(&self) -> TextList {
        self.text_pressed.clone()
    }

    /// Returns true if either Alt key is held
    #[inline]
    pub fn is_alt_down(&self) -> bool {
        self.is_down(KeyCode::AltLeft) || self.is_down(KeyCode::AltRight)
    }

    /// Returns true if either Ctrl key is held
    #[inline]
    pub fn is_crtl_down(&self) -> bool {
        self.is_down(KeyCode::ControlLeft) || self.is_down(KeyCode::ControlRight)
    }

    /// Returns true if either Shift key is held
    #[inline]
    pub fn is_shift_down(&self) -> bool {
        self.is_down(KeyCode::ShiftLeft) || self.is_down(KeyCode::ShiftRight)
    }

    /// Returns true if either Super/Cmd key is held
    #[inline]
    pub fn is_super_down(&self) -> bool {
        self.is_down(KeyCode::SuperLeft) || self.is_down(KeyCode::SuperRight)
    }

    /// Provides mutable access for manual state control, useful for input translation
    #[inline]
    pub fn mut_handle(&mut self) -> KeyboardMutHandle<'_> {
        KeyboardMutHandle { keyboard: self }
    }
}

/// Provides mutable access to keyboard state for manual control
pub struct KeyboardMutHandle<'a> {
    keyboard: &'a mut Keyboard,
}

impl<'a> KeyboardMutHandle<'a> {
    /// Marks a key as currently held
    #[inline]
    pub fn set_key_down(&mut self, key: KeyCode) {
        self.keyboard.key_down.insert(key);
    }

    /// Marks a key as not held
    #[inline]
    pub fn clear_key_down(&mut self, key: KeyCode) {
        self.keyboard.key_down.remove(key);
    }

    /// Registers a key press event for this frame
    #[inline]
    pub fn set_key_pressed(&mut self, key: KeyCode) {
        self.keyboard.key_pressed.insert(key);
    }

    /// Clears a key press event
    #[inline]
    pub fn clear_key_pressed(&mut self, key: KeyCode) {
        self.keyboard.key_pressed.remove(key);
    }

    /// Registers a key release event for this frame
    #[inline]
    pub fn set_key_released(&mut self, key: KeyCode) {
        self.keyboard.key_released.insert(key);
    }

    /// Clears a key release event
    #[inline]
    pub fn clear_key_released(&mut self, key: KeyCode) {
        self.keyboard.key_released.remove(key);
    }
}

fn init_keyboard_system(mut cmds: Commands) {
    cmds.insert_resource(Keyboard {
        key_pressed: keys_pressed(),
        key_down: keys_down(),
        key_released: keys_released(),
        text_pressed: text_pressed(),
    })
}

fn populate_keyboard_system(mut keyboard: ResMut<Keyboard>) {
    keyboard.key_down = keys_down();
    keyboard.key_pressed = keys_pressed();
    keyboard.key_released = keys_released();
    keyboard.text_pressed = text_pressed();
}
