#![cfg(feature = "gamepad")]
// TODO gamepad (web and native must have same API, also events onConnect, etc...)

// pub enum GamepadType {
//     Xbox,
//     Sony,
//     Nintendo,
//     Unknown,
// }

use crate::option_usize_env;
use crate::utils::{next_pot2, EnumSet};
use arrayvec::ArrayVec;
use nohash_hasher::IsEnabled;
use smallvec::SmallVec;
use std::hash::Hasher;
use strum::EnumCount;
use strum_macros::{EnumCount, FromRepr};

const GAMEPAD_BUTTON_COUNT_POT2: usize = next_pot2(GamepadButton::COUNT);

// Passing this env variable we can control the size of the hashset to reduce memory consume.
// 16 gamepads at once seems more than enough, most games have a max of 4-8, and other libs as
// SDL seems to allow a max of 16. still, we can pass as var in the build.rs a higher value
// if the game will require more gamepads
const MAX_GAMEPADS_CONNECTED: usize = option_usize_env!("GK_MAX_GAMEPADS_CONNECTED", 16);
pub(crate) const GAMEPADS_CONNECTED_POT2: usize = next_pot2(MAX_GAMEPADS_CONNECTED);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct GamepadId(pub(crate) usize);

impl GamepadId {
    pub(crate) fn raw(&self) -> usize {
        self.0
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumCount)]
#[repr(u8)]
pub enum GamepadButton {
    North,
    South,
    West,
    East,

    LeftShoulder,
    LeftTrigger,
    RightShoulder,
    RightTrigger,

    LeftStick,
    RightStick,

    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,

    Menu,
    Select,
    Start,

    Unknown,
}

#[derive(Default, Clone)]
pub struct GamepadButtonList {
    set: EnumSet<UniqueGamepadButton, GAMEPAD_BUTTON_COUNT_POT2>,
}

impl GamepadButtonList {
    pub fn insert(&mut self, v: GamepadButton) -> bool {
        self.set.insert(UniqueGamepadButton(v)).unwrap_or_default()
    }

    pub fn contains(&self, btn: GamepadButton) -> bool {
        self.set.contains(&UniqueGamepadButton(btn))
    }

    pub fn iter(&self) -> impl Iterator<Item = GamepadButton> + '_ {
        self.set.iter().map(|unique_btn| unique_btn.0)
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    pub fn remove(&mut self, btn: GamepadButton) -> bool {
        self.set.remove(&UniqueGamepadButton(btn))
    }

    pub fn clear(&mut self) {
        self.set.clear()
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct UniqueGamepadButton(GamepadButton);
impl std::hash::Hash for UniqueGamepadButton {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_u8(self.0 as _)
    }
}

impl IsEnabled for UniqueGamepadButton {}

impl std::fmt::Debug for GamepadButtonList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumCount, FromRepr)]
#[repr(u8)]
pub enum GamepadAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,

    RightTrigger,
    LeftTrigger,

    Unknown,
}

#[derive(Default, Clone)]
pub struct GamepadAxisList {
    list: [f32; GamepadAxis::COUNT],
}

impl GamepadAxisList {
    pub fn set_strength(&mut self, axis: GamepadAxis, force: f32) {
        self.list[axis as usize] = force;
    }

    pub fn strength(&self, axis: GamepadAxis) -> f32 {
        self.list[axis as usize]
    }

    pub fn iter(&self) -> impl Iterator<Item = (GamepadAxis, f32)> + '_ {
        self.list
            .iter()
            .enumerate()
            .map(|(i, &f)| (GamepadAxis::from_repr(i as _).unwrap(), f))
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn clear(&mut self) {
        self.list = Default::default();
    }
}

impl std::fmt::Debug for GamepadAxisList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

#[derive(Debug, Default, Clone)]
pub struct GamepadInfo {
    id: usize,

    pub pressed: GamepadButtonList,
    pub down: GamepadButtonList,
    pub released: GamepadButtonList,

    pub axis: GamepadAxisList,
}

impl GamepadInfo {
    pub fn press(&mut self, btn: GamepadButton) {
        self.pressed.insert(btn);
        self.down.insert(btn);
        self.released.remove(btn);
    }

    pub fn release(&mut self, btn: GamepadButton) {
        self.released.insert(btn);
        self.down.remove(btn);
        self.pressed.remove(btn);
    }

    pub fn set_axis_strength(&mut self, axis: GamepadAxis, strength: f32) {
        self.axis.set_strength(axis, strength);
    }

    pub fn axis_strength(&self, axis: GamepadAxis) -> f32 {
        self.axis.strength(axis)
    }

    pub fn are_pressed<const N: usize>(&self, btns: &[GamepadButton; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_pressed(btn);
        });
        res
    }

    pub fn is_pressed(&self, btn: GamepadButton) -> bool {
        self.pressed.contains(btn)
    }

    pub fn are_released<const N: usize>(&self, btns: &[GamepadButton; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_released(btn);
        });
        res
    }

    pub fn is_released(&self, btn: GamepadButton) -> bool {
        self.released.contains(btn)
    }

    pub fn are_down<const N: usize>(&self, btns: &[GamepadButton; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_down(btn);
        });
        res
    }

    pub fn is_down(&self, btn: GamepadButton) -> bool {
        self.down.contains(btn)
    }

    pub fn tick(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }
}

#[derive(Default, Clone)]
pub struct GamepadList {
    ids: ArrayVec<GamepadId, GAMEPADS_CONNECTED_POT2>,
}

impl GamepadList {
    pub fn iter(&self) -> impl Iterator<Item = &GamepadId> + '_ {
        self.ids.iter()
    }

    pub fn into_iter<'a>(self) -> impl IntoIterator<Item = GamepadId> + 'a {
        self.ids.into_iter()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn clear(&mut self) {
        self.ids.clear();
    }
}

impl std::fmt::Debug for GamepadList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct GamepadState {
    pub gamepads: ArrayVec<GamepadInfo, GAMEPADS_CONNECTED_POT2>,
}

impl GamepadState {
    pub fn available(&self) -> GamepadList {
        GamepadList {
            ids: self
                .gamepads
                .iter()
                .map(|info| GamepadId(info.id))
                .collect::<ArrayVec<_, GAMEPADS_CONNECTED_POT2>>(),
        }
    }

    pub fn add(&mut self, id: usize) {
        self.gamepads.push(GamepadInfo {
            id,
            ..Default::default()
        });
    }

    pub fn remove(&mut self, id: usize) {
        let idx = self.gamepads.iter().position(|info| info.id == id);
        if let Some(idx) = idx {
            self.gamepads.remove(idx);
        }
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut GamepadInfo> {
        self.gamepads.iter_mut().find(|info| info.id == id)
    }

    pub fn get(&self, id: usize) -> Option<&GamepadInfo> {
        self.gamepads.iter().find(|info| info.id == id)
    }

    pub fn tick(&mut self) {
        self.gamepads.iter_mut().for_each(|info| info.tick());
    }
}

// FIXME strum FromRepr seems to not work fine with test, it fails compiling because it cannot find option...
// #[cfg(test)]
// mod tests {
// use super::*;
//
// #[test]
// fn test_add_remove_gamepads() {
//     let mut gamepad_state = GamepadState::default();
//     gamepad_state.add(1);
//     gamepad_state.add(2);
//
//     assert_eq!(gamepad_state.gamepads.len(), 2);
//
//     gamepad_state.remove(1);
//     assert_eq!(gamepad_state.gamepads.len(), 1);
//
//     assert!(gamepad_state.get(1).is_none());
//     assert!(gamepad_state.get(2).is_some());
// }
//
// #[test]
// fn test_press_release_buttons() {
//     let mut gamepad_info = GamepadInfo::default();
//
//     gamepad_info.press(GamepadButton::North);
//     assert!(gamepad_info.is_pressed(GamepadButton::North));
//     assert!(gamepad_info.is_down(GamepadButton::North));
//     assert!(!gamepad_info.is_released(GamepadButton::North));
//
//     gamepad_info.release(GamepadButton::North);
//     assert!(!gamepad_info.is_pressed(GamepadButton::North));
//     assert!(!gamepad_info.is_down(GamepadButton::North));
//     assert!(gamepad_info.is_released(GamepadButton::North));
// }
//
// #[test]
// fn test_axis_strength() {
//     let mut gamepad_info = GamepadInfo::default();
//
//     gamepad_info.set_axis_strength(GamepadAxis::LeftX, 0.5);
//     assert_eq!(gamepad_info.axis_strength(GamepadAxis::LeftX), 0.5);
//
//     gamepad_info.set_axis_strength(GamepadAxis::RightY, -1.0);
//     assert_eq!(gamepad_info.axis_strength(GamepadAxis::RightY), -1.0);
//
//     // Test default value (should be 0.0 for uninitialized axis)
//     assert_eq!(gamepad_info.axis_strength(GamepadAxis::LeftY), 0.0);
// }
//
// #[test]
// fn test_multiple_buttons() {
//     let mut gamepad_info = GamepadInfo::default();
//
//     gamepad_info.press(GamepadButton::South);
//     gamepad_info.press(GamepadButton::East);
//
//     assert!(gamepad_info.is_pressed(GamepadButton::South));
//     assert!(gamepad_info.is_pressed(GamepadButton::East));
//     assert!(gamepad_info.is_down(GamepadButton::South));
//     assert!(gamepad_info.is_down(GamepadButton::East));
//
//     gamepad_info.release(GamepadButton::South);
//     assert!(!gamepad_info.is_pressed(GamepadButton::South));
//     assert!(gamepad_info.is_pressed(GamepadButton::East));
//
//     gamepad_info.tick();
//     assert!(!gamepad_info.is_pressed(GamepadButton::South));
//     assert!(!gamepad_info.is_pressed(GamepadButton::East));
//     assert!(gamepad_info.is_down(GamepadButton::East));
//     assert!(gamepad_info.is_released(GamepadButton::South));
// }
//
// #[test]
// fn test_unknown_buttons_axes() {
//     let mut gamepad_info = GamepadInfo::default();
//
//     gamepad_info.press(GamepadButton::Unknown);
//     assert!(gamepad_info.is_pressed(GamepadButton::Unknown));
//
//     gamepad_info.set_axis_strength(GamepadAxis::Unknown, 0.7);
//     assert_eq!(gamepad_info.axis_strength(GamepadAxis::Unknown), 0.7);
// }
//
// #[test]
// fn test_clear_gamepad_state() {
//     let mut gamepad_info = GamepadInfo::default();
//
//     gamepad_info.press(GamepadButton::North);
//     gamepad_info.set_axis_strength(GamepadAxis::LeftX, 0.5);
//     gamepad_info.tick(); // Simulate a frame tick to clear state
//
//     assert!(!gamepad_info.is_pressed(GamepadButton::North));
//     assert_eq!(gamepad_info.axis_strength(GamepadAxis::LeftX), 0.5);
// }
//
// #[test]
// fn test_iterate_gamepads() {
//     let mut gamepad_state = GamepadState::default();
//     gamepad_state.add(1);
//     gamepad_state.add(2);
//
//     let gamepad_list = gamepad_state.available();
//     let ids: Vec<usize> = gamepad_list.iter().map(|id| id.raw()).collect();
//
//     assert_eq!(ids, vec![1, 2]);
// }
// }
