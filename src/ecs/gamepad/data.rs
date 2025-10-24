use std::hash::Hasher;

use nohash_hasher::{BuildNoHashHasher, IsEnabled};
use strum::EnumCount;
use strum_macros::{EnumCount, EnumIter, FromRepr};
use uuid::Uuid;

const GAMEPAD_BUTTON_COUNT_POT2: usize = GamepadButton::COUNT.next_power_of_two();

/// Standard gamepad button layout (North=Y/Triangle, South=A/Cross, etc.)
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

impl GamepadButton {
    /// Get the vendor-specific name for this button
    pub fn vendor_name(self, vendor: GamepadVendor) -> &'static str {
        use GamepadButton as B;
        match vendor {
            GamepadVendor::PlayStation => match self {
                B::South => "Cross",
                B::East => "Circle",
                B::West => "Square",
                B::North => "Triangle",

                B::LeftShoulder => "L1",
                B::LeftTrigger => "L2",
                B::RightShoulder => "R1",
                B::RightTrigger => "R2",

                B::LeftStick => "L3",
                B::RightStick => "R3",

                B::DPadUp => "D-Pad Up",
                B::DPadDown => "D-Pad Down",
                B::DPadLeft => "D-Pad Left",
                B::DPadRight => "D-Pad Right",

                B::Start => "Options",
                B::Select => "Create",
                B::Menu => "PS",

                B::Unknown => "Unknown",
            },

            GamepadVendor::Nintendo => match self {
                B::South => "B",
                B::East => "A",
                B::West => "Y",
                B::North => "X",

                B::LeftShoulder => "L",
                B::LeftTrigger => "ZL",
                B::RightShoulder => "R",
                B::RightTrigger => "ZR",

                B::LeftStick => "L Stick",
                B::RightStick => "R Stick",

                B::DPadUp => "D-Pad Up",
                B::DPadDown => "D-Pad Down",
                B::DPadLeft => "D-Pad Left",
                B::DPadRight => "D-Pad Right",

                B::Start => "Plus",
                B::Select => "Minus",
                B::Menu => "Home",

                B::Unknown => "Unknown",
            },

            GamepadVendor::Xbox | GamepadVendor::Unknown => match self {
                B::South => "A",
                B::East => "B",
                B::West => "X",
                B::North => "Y",

                B::LeftShoulder => "LB",
                B::LeftTrigger => "LT",
                B::RightShoulder => "RB",
                B::RightTrigger => "RT",

                B::LeftStick => "LS",
                B::RightStick => "RS",

                B::DPadUp => "D-Pad Up",
                B::DPadDown => "D-Pad Down",
                B::DPadLeft => "D-Pad Left",
                B::DPadRight => "D-Pad Right",

                B::Start => "Menu",
                B::Select => "View",
                B::Menu => "Guide",

                B::Unknown => "Unknown",
            },
        }
    }
}

/// Collection of gamepad buttons
#[derive(Default, Clone)]
pub struct GamepadButtonList {
    set: heapless::index_set::IndexSet<
        UniqueGamepadButton,
        BuildNoHashHasher<UniqueGamepadButton>,
        GAMEPAD_BUTTON_COUNT_POT2,
    >,
}

impl GamepadButtonList {
    pub(super) fn insert(&mut self, v: GamepadButton) -> bool {
        self.set.insert(UniqueGamepadButton(v)).unwrap_or_default()
    }

    /// Check if a button is in this list
    pub fn contains(&self, btn: GamepadButton) -> bool {
        self.set.contains(&UniqueGamepadButton(btn))
    }

    /// Iterate over all buttons in this list
    pub fn iter(&self) -> impl Iterator<Item = GamepadButton> + '_ {
        self.set.iter().map(|unique_btn| unique_btn.0)
    }

    /// Get the number of buttons in this list
    pub fn len(&self) -> usize {
        self.set.len()
    }

    /// Check if this list is empty
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    /// Remove a button from this list
    pub fn remove(&mut self, btn: GamepadButton) -> bool {
        self.set.remove(&UniqueGamepadButton(btn))
    }

    /// Clear all buttons from this list
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

/// Standard gamepad axis layout (sticks and triggers)
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumCount, FromRepr, EnumIter,
)]
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

impl GamepadAxis {
    /// Get the vendor-specific name for this axis
    pub fn vendor_name(self, vendor: GamepadVendor) -> &'static str {
        use GamepadAxis as A;
        match vendor {
            GamepadVendor::PlayStation => match self {
                A::LeftX => "Left Stick X",
                A::LeftY => "Left Stick Y",
                A::RightX => "Right Stick X",
                A::RightY => "Right Stick Y",
                A::LeftTrigger => "L2",
                A::RightTrigger => "R2",
                A::Unknown => "Unknown",
            },

            GamepadVendor::Nintendo => match self {
                A::LeftX => "Left Stick X",
                A::LeftY => "Left Stick Y",
                A::RightX => "Right Stick X",
                A::RightY => "Right Stick Y",
                A::LeftTrigger => "ZL",
                A::RightTrigger => "ZR",
                A::Unknown => "Unknown",
            },

            GamepadVendor::Xbox | GamepadVendor::Unknown => match self {
                A::LeftX => "Left Stick X",
                A::LeftY => "Left Stick Y",
                A::RightX => "Right Stick X",
                A::RightY => "Right Stick Y",
                A::LeftTrigger => "LT",
                A::RightTrigger => "RT",
                A::Unknown => "Unknown",
            },
        }
    }
}

/// Collection of gamepad axis
#[derive(Default, Clone)]
pub struct GamepadAxisList {
    list: [f32; GamepadAxis::COUNT],
}

impl GamepadAxisList {
    pub(super) fn set_strength(&mut self, axis: GamepadAxis, force: f32) {
        self.list[axis as usize] = force;
    }

    /// Get the current value of an axis (-1.0 to 1.0)
    pub fn strength(&self, axis: GamepadAxis) -> f32 {
        self.list[axis as usize]
    }

    /// Iterate over all axis values
    pub fn iter(&self) -> impl Iterator<Item = (GamepadAxis, f32)> + '_ {
        self.list
            .iter()
            .enumerate()
            .map(|(i, &f)| (GamepadAxis::from_repr(i as _).unwrap(), f))
    }

    /// Get the number of axes
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Empty if all axes are at neutral position
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Reset all axes to default values
    pub fn clear(&mut self) {
        self.list = Default::default();
    }
}

impl std::fmt::Debug for GamepadAxisList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// Gamepad vendor names
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum GamepadVendor {
    #[default]
    Unknown,
    Xbox,
    PlayStation,
    Nintendo,
}

pub(super) fn detect_vendor(gp: &gilrs::Gamepad) -> GamepadVendor {
    // let try to detect the vendor from the vendor id
    let vendor = gp.vendor_id().and_then(|vid| match vid {
        0x045E => Some(GamepadVendor::Xbox),
        0x054C => Some(GamepadVendor::PlayStation),
        0x057E => Some(GamepadVendor::Nintendo),
        _ => None,
    });

    if let Some(vendor) = vendor {
        return vendor;
    }

    // if we failed to detect the vendor from the vendor id
    // let's try to detect it from the name
    let map = gp.map_name().unwrap_or("");
    let name = gp.name();
    let osn = gp.os_name();

    let any = |s: &str, needles: &[&str]| {
        let s = s.to_ascii_lowercase();
        needles.iter().any(|n| s.contains(n))
    };

    if any(map, &["xbox"]) || any(name, &["xbox"]) || any(osn, &["xbox", "xinput"]) {
        return GamepadVendor::Xbox;
    }
    if any(
        map,
        &["dualsense", "dualshock", "playstation", "ps4", "ps5"],
    ) || any(
        name,
        &["dualsense", "dualshock", "playstation", "ps4", "ps5"],
    ) || any(osn, &["dualsense", "dualshock", "playstation"])
    {
        return GamepadVendor::PlayStation;
    }
    if any(map, &["nintendo", "switch", "joy-con", "pro controller"])
        || any(name, &["nintendo", "switch", "joy-con", "pro controller"])
        || any(osn, &["nintendo", "switch", "joy-con", "pro controller"])
    {
        return GamepadVendor::Nintendo;
    }

    GamepadVendor::Unknown
}

pub(super) struct GamepadInfo {
    pub id: gilrs::GamepadId,
    pub name: String,
    pub uuid: Uuid,
    pub vendor: GamepadVendor,

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

    pub fn is_pressed(&self, btn: GamepadButton) -> bool {
        self.pressed.contains(btn)
    }

    pub fn is_released(&self, btn: GamepadButton) -> bool {
        self.released.contains(btn)
    }

    pub fn is_down(&self, btn: GamepadButton) -> bool {
        self.down.contains(btn)
    }

    pub fn tick(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_button_list_insert_contains() {
        let mut list = GamepadButtonList::default();
        assert!(!list.contains(GamepadButton::North));

        list.insert(GamepadButton::North);
        assert!(list.contains(GamepadButton::North));
    }

    #[test]
    fn test_button_list_remove() {
        let mut list = GamepadButtonList::default();
        list.insert(GamepadButton::South);
        assert!(list.contains(GamepadButton::South));

        list.remove(GamepadButton::South);
        assert!(!list.contains(GamepadButton::South));
    }

    #[test]
    fn test_button_list_len_and_empty() {
        let mut list = GamepadButtonList::default();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);

        list.insert(GamepadButton::West);
        assert!(!list.is_empty());
        assert_eq!(list.len(), 1);

        list.insert(GamepadButton::East);
        assert_eq!(list.len(), 2);

        list.remove(GamepadButton::West);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_button_list_iter() {
        let mut list = GamepadButtonList::default();
        list.insert(GamepadButton::LeftShoulder);
        list.insert(GamepadButton::RightShoulder);

        let buttons: Vec<_> = list.iter().collect();
        assert_eq!(buttons.len(), 2);
        assert!(buttons.contains(&GamepadButton::LeftShoulder));
        assert!(buttons.contains(&GamepadButton::RightShoulder));
    }

    #[test]
    fn test_button_list_clear() {
        let mut list = GamepadButtonList::default();
        list.insert(GamepadButton::Start);
        list.insert(GamepadButton::Select);
        assert_eq!(list.len(), 2);

        list.clear();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_axis_list_strength() {
        let mut list = GamepadAxisList::default();
        list.set_strength(GamepadAxis::LeftX, 1.0);
        assert_eq!(list.strength(GamepadAxis::LeftX), 1.0);

        list.set_strength(GamepadAxis::LeftY, -0.5);
        assert_eq!(list.strength(GamepadAxis::LeftY), -0.5);

        list.set_strength(GamepadAxis::RightX, 0.25);
        assert_eq!(list.strength(GamepadAxis::RightX), 0.25);
    }

    #[test]
    fn test_axis_list_iter() {
        let mut list = GamepadAxisList::default();
        list.set_strength(GamepadAxis::LeftX, 1.0);
        list.set_strength(GamepadAxis::RightY, -0.5);

        let axes: Vec<_> = list.iter().collect();
        assert_eq!(axes.len(), GamepadAxis::COUNT);

        let left_x = axes
            .iter()
            .find(|(axis, _)| *axis == GamepadAxis::LeftX)
            .unwrap();
        assert_eq!(left_x.1, 1.0);

        let right_y = axes
            .iter()
            .find(|(axis, _)| *axis == GamepadAxis::RightY)
            .unwrap();
        assert_eq!(right_y.1, -0.5);
    }

    #[test]
    fn test_axis_list_clear() {
        let mut list = GamepadAxisList::default();
        list.set_strength(GamepadAxis::LeftX, 1.0);
        list.set_strength(GamepadAxis::RightY, -0.5);
        assert_eq!(list.strength(GamepadAxis::LeftX), 1.0);
        assert_eq!(list.strength(GamepadAxis::RightY), -0.5);

        list.clear();
        assert_eq!(list.strength(GamepadAxis::LeftX), 0.0);
        assert_eq!(list.strength(GamepadAxis::RightY), 0.0);
    }
}
