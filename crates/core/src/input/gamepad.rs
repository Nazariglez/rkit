// TODO gamepad (web and native must have same API, also events onConnect, etc...)

// pub enum GamepadType {
//     Xbox,
//     Sony,
//     Nintendo,
//     Unknown,
// }

use strum_macros::EnumCount;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumCount)]
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

#[derive(Clone, Debug, Default)]
pub struct GamepadState {}
