use crate::math::Vec2;
use crate::utils::EnumSet;
use nohash_hasher::IsEnabled;
use std::hash::Hasher;
use strum::EnumCount;
use strum_macros::EnumIter;

#[derive(
    Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, strum_macros::EnumCount, EnumIter,
)]
#[repr(u8)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Back,
    Forward,

    // Unknown is the last value
    Unknown,
}

const MOUSE_BUTTON_COUNT_POT2: usize = MouseButton::COUNT.next_power_of_two();

#[derive(Default, Clone)]
pub struct MouseButtonList {
    set: EnumSet<UniqueMouseButton, MOUSE_BUTTON_COUNT_POT2>,
}

impl MouseButtonList {
    pub fn insert(&mut self, v: MouseButton) -> bool {
        self.set.insert(UniqueMouseButton(v)).unwrap_or_default()
    }

    pub fn contains(&self, btn: MouseButton) -> bool {
        self.set.contains(&UniqueMouseButton(btn))
    }

    pub fn iter(&self) -> impl Iterator<Item = MouseButton> + '_ {
        self.set.iter().map(|unique_btn| unique_btn.0)
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    pub fn remove(&mut self, btn: MouseButton) -> bool {
        self.set.remove(&UniqueMouseButton(btn))
    }

    pub fn clear(&mut self) {
        self.set.clear()
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct UniqueMouseButton(MouseButton);
impl std::hash::Hash for UniqueMouseButton {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_u8(self.0 as _)
    }
}

impl IsEnabled for UniqueMouseButton {}

impl std::fmt::Debug for MouseButtonList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MouseState {
    pub position: Vec2,
    pub motion_delta: Vec2,
    pub wheel_delta: Vec2,
    pub moving: bool,
    pub scrolling: bool,
    pub cursor_on_screen: bool,

    pub pressed: MouseButtonList,
    pub released: MouseButtonList,
    pub down: MouseButtonList,
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
        if self.down.contains(btn) {
            return;
        }

        self.pressed.insert(btn);
        self.down.insert(btn);
        self.released.remove(btn);
    }

    pub fn release(&mut self, btn: MouseButton) {
        if !self.down.contains(btn) {
            return;
        }

        self.released.insert(btn);
        self.down.remove(btn);
        self.pressed.remove(btn);
    }

    pub fn are_pressed<const N: usize>(&self, btns: &[MouseButton; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_pressed(btn);
        });
        res
    }

    pub fn is_pressed(&self, btn: MouseButton) -> bool {
        self.pressed.contains(btn)
    }

    pub fn are_released<const N: usize>(&self, btns: &[MouseButton; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_released(btn);
        });
        res
    }

    pub fn is_released(&self, btn: MouseButton) -> bool {
        self.released.contains(btn)
    }

    pub fn are_down<const N: usize>(&self, btns: &[MouseButton; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_down(btn);
        });
        res
    }

    pub fn is_down(&self, btn: MouseButton) -> bool {
        self.down.contains(btn)
    }

    pub fn is_cursor_on_screen(&self) -> bool {
        self.cursor_on_screen
    }

    pub fn tick(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.motion_delta = Vec2::ZERO;
        self.moving = false;
        self.wheel_delta = Vec2::ZERO;
        self.scrolling = false;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::math::vec2;

    #[test]
    fn test_list_insert_contains() {
        let mut list = MouseButtonList::default();
        assert!(!list.contains(MouseButton::Left));

        list.insert(MouseButton::Left);
        assert!(list.contains(MouseButton::Left));
    }

    #[test]
    fn test_list_remove() {
        let mut list = MouseButtonList::default();
        list.insert(MouseButton::Right);
        assert!(list.contains(MouseButton::Right));

        list.remove(MouseButton::Right);
        assert!(!list.contains(MouseButton::Right));
    }

    #[test]
    fn test_list_len_and_empty() {
        let mut list = MouseButtonList::default();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);

        list.insert(MouseButton::Middle);
        assert!(!list.is_empty());
        assert_eq!(list.len(), 1);

        list.insert(MouseButton::Left);
        assert_eq!(list.len(), 2);

        list.remove(MouseButton::Middle);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_list_iter() {
        let mut list = MouseButtonList::default();
        list.insert(MouseButton::Back);
        list.insert(MouseButton::Forward);

        let buttons: Vec<_> = list.iter().collect();
        assert_eq!(buttons.len(), 2);
        assert!(buttons.contains(&MouseButton::Back));
        assert!(buttons.contains(&MouseButton::Forward));
    }

    #[test]
    fn test_state_press_and_release() {
        let mut state = MouseState::default();

        assert!(!state.is_pressed(MouseButton::Left));
        assert!(!state.is_down(MouseButton::Left));
        assert!(!state.is_released(MouseButton::Left));

        state.press(MouseButton::Left);
        assert!(state.is_pressed(MouseButton::Left));
        assert!(state.is_down(MouseButton::Left));
        assert!(!state.is_released(MouseButton::Left));

        state.release(MouseButton::Left);
        assert!(!state.is_pressed(MouseButton::Left));
        assert!(!state.is_down(MouseButton::Left));
        assert!(state.is_released(MouseButton::Left));
    }

    #[test]
    fn test_state_are_pressed_are_released_are_down() {
        let mut state = MouseState::default();
        state.press(MouseButton::Left);
        state.press(MouseButton::Right);

        let pressed =
            state.are_pressed(&[MouseButton::Left, MouseButton::Middle, MouseButton::Right]);
        assert_eq!(pressed, [true, false, true]);

        state.release(MouseButton::Right);

        let released = state.are_released(&[MouseButton::Left, MouseButton::Right]);
        assert_eq!(released, [false, true]);

        let down = state.are_down(&[MouseButton::Left, MouseButton::Right]);
        assert_eq!(down, [true, false]);
    }

    #[test]
    fn test_state_tick() {
        let mut state = MouseState::default();
        state.press(MouseButton::Left);
        state.press(MouseButton::Right);
        state.motion_delta = vec2(5.0, 5.0);
        state.moving = true;
        state.wheel_delta = vec2(1.0, 1.0);
        state.scrolling = true;

        state.tick();

        assert!(state.pressed.is_empty());
        assert!(state.released.is_empty());
        assert_eq!(state.down.len(), 2);
        assert_eq!(state.motion_delta, Vec2::ZERO);
        assert!(!state.moving);
        assert_eq!(state.wheel_delta, Vec2::ZERO);
        assert!(!state.scrolling);
    }
}
