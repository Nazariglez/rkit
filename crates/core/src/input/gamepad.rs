// TODO gamepad (web and native must have same API, also events onConnect, etc...)

// pub enum GamepadType {
//     Xbox,
//     Sony,
//     Nintendo,
//     Unknown,
// }

use crate::utils::{next_pot2, EnumSet};
use nohash_hasher::IsEnabled;
use std::hash::Hasher;
use strum::EnumCount;
use strum_macros::{EnumCount, FromRepr};

const GAMEPAD_BUTTON_COUNT_POT2: usize = next_pot2(GamepadButton::COUNT);

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

#[derive(Clone, Debug, Default)]
pub struct GamepadState {
    // TODO devices available
    pub pressed: GamepadButtonList,
    pub down: GamepadButtonList,
    pub released: GamepadButtonList,

    pub axis: GamepadAxisList,
}

impl GamepadState {}
