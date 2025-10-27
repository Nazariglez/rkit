use super::data::*;
use crate::ecs::bevy_ecs::prelude::*;
use core::slice;
use gilrs::{Axis, Button, Event, EventType, GamepadId as GilrsGamepadId, Gilrs};
use heapless::index_map::FnvIndexMap;
use uuid::Uuid;

// Passing this env variable we can control the size of the hashset to reduce memory consume.
// 16 gamepads at once seems more than enough, most games have a max of 4-8, and other libs as
// SDL seems to allow a max of 16.
const MAX_GAMEPADS: usize = 16;

/// A slot identifier for a gamepad
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GamepadSlot(u8);

impl From<u8> for GamepadSlot {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<GamepadSlot> for u8 {
    fn from(value: GamepadSlot) -> Self {
        value.0
    }
}

/// Manages all connected gamepads and their states
#[derive(Default, Resource)]
pub struct Gamepads {
    slots: [Option<GamepadInfo>; MAX_GAMEPADS],
    ids: FnvIndexMap<GilrsGamepadId, GamepadSlot, MAX_GAMEPADS>,
}

impl Gamepads {
    /// Check if a gamepad is connected in the given slot
    #[inline]
    pub fn is_connected(&self, slot: impl Into<GamepadSlot>) -> bool {
        let slot: GamepadSlot = slot.into();
        self.slots[slot.0 as usize].is_some()
    }

    /// Iterate over all gamepad slots (connected and empty)
    #[inline]
    pub fn iter_slots(&self) -> GamepadsIter<'_> {
        GamepadsIter {
            enumerated: self.slots.iter().enumerate(),
        }
    }

    /// Iterate over only connected gamepads
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = GamepadState<'_>> {
        self.slots.iter().enumerate().filter_map(|(i, opt)| {
            opt.as_ref().map(|info| GamepadState {
                slot: GamepadSlot(i as u8),
                info: Some(info),
            })
        })
    }

    /// Get gamepad state for a specific slot
    #[inline]
    pub fn get(&self, slot: impl Into<GamepadSlot>) -> GamepadState<'_> {
        let s = slot.into();
        GamepadState {
            slot: s,
            info: self.slots[s.0 as usize].as_ref(),
        }
    }

    fn tick(&mut self) {
        for slot in &mut self.slots {
            if let Some(info) = slot {
                info.tick();
            }
        }
    }

    fn add_gamepad(&mut self, info: GamepadInfo) -> Option<GamepadSlot> {
        let name = info.name.clone();
        let id = info.id.clone();
        let vendor = info.vendor;
        let slot_idx = self.slots.iter().position(|slot| slot.is_none())?;
        self.slots[slot_idx] = Some(info);

        log::info!("Gamepad '{name}' ({vendor:?}) connected on slot {slot_idx}");
        let slot = GamepadSlot(slot_idx as u8);
        self.ids.insert(id, slot).unwrap();
        Some(slot)
    }

    fn remove_gamepad(&mut self, id: GilrsGamepadId) -> Option<(GamepadSlot, GamepadInfo)> {
        self.ids.remove(&id).and_then(|slot| {
            let info = self.slots[slot.0 as usize].take();
            debug_assert!(info.is_some());

            match info {
                Some(info) => {
                    log::info!(
                        "Gamepad '{}' ({:?}) disconnected from slot {}",
                        info.name,
                        info.vendor,
                        slot.0
                    );
                    Some((slot, info))
                }
                None => {
                    log::error!("Gamepad disconnected but not found in slots");
                    None
                }
            }
        })
    }

    fn slot_mut(&mut self, id: GilrsGamepadId) -> Option<&mut GamepadInfo> {
        self.ids
            .get(&id)
            .copied()
            .and_then(|slot| self.slots[slot.0 as usize].as_mut())
    }

    #[inline(always)]
    fn contains_raw(&self, id: GilrsGamepadId) -> bool {
        self.ids.contains_key(&id)
    }
}

pub(super) struct RawGilrs(Gilrs);

impl RawGilrs {
    pub fn new() -> Result<Self, String> {
        let gilrs = Gilrs::new().map_err(|e| e.to_string())?;
        Ok(Self(gilrs))
    }
}

#[inline(always)]
fn register_gamepad_if_necessary(gamepads: &mut Gamepads, raw: &mut RawGilrs, id: GilrsGamepadId) {
    if gamepads.contains_raw(id) {
        return;
    }

    let info = raw.0.gamepad(id);
    let uuid = Uuid::from_bytes(info.uuid());
    let vendor = detect_vendor(&info);
    let slot = gamepads.add_gamepad(GamepadInfo {
        id,
        name: info.name().to_string(),
        uuid,
        vendor,
        pressed: Default::default(),
        down: Default::default(),
        released: Default::default(),
        axis: Default::default(),
    });
    match slot {
        Some(slot) => {
            log::debug!(
                "Gamepad connected '{:?}': raw_id={:?}, name={}, uuid={}, vendor={:?}",
                slot,
                id,
                info.name(),
                uuid,
                vendor,
            );
        }
        None => {
            log::warn!(
                "Gamepad connection ignored, not enoguh slots. raw_id='{:?}', name='{}', uuid='{:?}', vendor='{:?}'",
                info.id(),
                info.name(),
                uuid,
                vendor,
            );
        }
    }
}

pub(super) fn sync_gilrs_events_system(
    mut gamepads: ResMut<Gamepads>,
    mut raw: NonSendMut<RawGilrs>,
) {
    gamepads.tick();

    while let Some(Event { id, event, .. }) = raw.0.next_event() {
        // FIXME: on macos if the gamepad is connected via usb only connect and disconnect events are received,
        // at least using a xbox controller. Probably I need to use Apples GameController framework under the hood
        // on mac to give full support to gamepads, but for now I guess that mac is not a priority and I can live
        // with it, just telling the user to use a bluetooth controller.

        match event {
            EventType::ButtonPressed(btn, _) => {
                register_gamepad_if_necessary(&mut gamepads, &mut raw, id);
                let info = gamepads.slot_mut(id);
                debug_assert!(info.is_some(), "Gamepad '{}' not registered?", id);
                if let Some(info) = info {
                    info.press(button_cast(btn));
                }
            }
            EventType::ButtonRepeated(..) => {}
            EventType::ButtonReleased(btn, _) => {
                register_gamepad_if_necessary(&mut gamepads, &mut raw, id);
                let info = gamepads.slot_mut(id);
                debug_assert!(info.is_some(), "Gamepad '{}' not registered?", id);
                if let Some(info) = info {
                    info.release(button_cast(btn));
                }
            }
            EventType::ButtonChanged(btn, strength, _) => {
                let mut cast_to_axis = |axis: GamepadAxis| {
                    register_gamepad_if_necessary(&mut gamepads, &mut raw, id);
                    let info = gamepads.slot_mut(id);
                    debug_assert!(info.is_some(), "Gamepad '{}' not registered?", id);
                    if let Some(info) = info {
                        info.set_axis_strength(axis, strength);
                    }
                };
                match btn {
                    Button::LeftTrigger2 => {
                        cast_to_axis(GamepadAxis::LeftTrigger);
                    }
                    Button::RightTrigger2 => {
                        cast_to_axis(GamepadAxis::RightTrigger);
                    }
                    _ => {}
                }
            }
            EventType::AxisChanged(axis, strength, _) => {
                register_gamepad_if_necessary(&mut gamepads, &mut raw, id);
                let info = gamepads.slot_mut(id);
                debug_assert!(info.is_some(), "Gamepad '{}' not registered?", id);
                if let Some(info) = info {
                    info.set_axis_strength(axis_cast(axis), strength);
                }
            }
            EventType::Connected => {
                register_gamepad_if_necessary(&mut gamepads, &mut raw, id);
            }
            EventType::Disconnected => {
                let info = gamepads.remove_gamepad(id);
                match info {
                    Some((slot, info)) => {
                        log::debug!(
                            "Gamepad disconnected '{slot:?}': raw_id={:?}, name='{}', uuid='{:?}', vendor='{:?}'",
                            id,
                            info.name,
                            info.uuid,
                            info.vendor
                        );
                    }
                    None => {
                        let info = raw.0.gamepad(id);
                        log::warn!(
                            "Gamepad disconnection ignored, not found. raw_id='{:?}', name='{}', uuid='{:?}', vendor='{:?}'",
                            id,
                            info.name(),
                            info.uuid(),
                            detect_vendor(&info)
                        );
                    }
                }
            }
            EventType::Dropped => {}
            EventType::ForceFeedbackEffectCompleted => {}
            _ => {}
        }
    }

    raw.0.inc();
}

/// Represents the state of a single gamepad
#[derive(Clone, Copy)]
pub struct GamepadState<'a> {
    slot: GamepadSlot,
    info: Option<&'a GamepadInfo>,
}

impl<'a> GamepadState<'a> {
    /// Check if this gamepad is connected
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.info.is_some()
    }
    /// Get the slot number for this gamepad
    #[inline]
    pub fn slot(&self) -> GamepadSlot {
        self.slot
    }
    /// Get the gamepad's device name
    #[inline]
    pub fn name(&self) -> Option<&'a str> {
        self.info.map(|i| i.name.as_str())
    }
    /// Get the gamepad's unique identifier
    #[inline]
    pub fn uuid(&self) -> Option<&'a Uuid> {
        self.info.map(|i| &i.uuid)
    }
    /// Get the gamepad's vendor (Xbox, PlayStation, etc.)
    #[inline]
    pub fn vendor(&self) -> Option<GamepadVendor> {
        self.info.map(|i| i.vendor)
    }
    /// Check if a button was just pressed this frame
    #[inline]
    pub fn just_pressed(&self, btn: GamepadButton) -> bool {
        self.info.map(|i| i.is_pressed(btn)).unwrap_or(false)
    }
    /// Check if a button was just released this frame
    #[inline]
    pub fn just_released(&self, btn: GamepadButton) -> bool {
        self.info.map(|i| i.is_released(btn)).unwrap_or(false)
    }
    /// Check if a button is currently held down
    #[inline]
    pub fn is_down(&self, btn: GamepadButton) -> bool {
        self.info.map(|i| i.is_down(btn)).unwrap_or(false)
    }
    /// Get the current value of an axis (-1.0 to 1.0)
    /// Left/Right trigger goes from 0.0 to 1.0
    #[inline]
    pub fn axis_movement(&self, axis: GamepadAxis) -> f32 {
        self.info.map(|i| i.axis_strength(axis)).unwrap_or(0.0)
    }
    /// Get all buttons currently held down
    #[inline]
    pub fn down_buttons(&self) -> GamepadButtonList {
        self.info.map(|i| i.down.clone()).unwrap_or_default()
    }
    /// Get all buttons pressed this frame
    #[inline]
    pub fn pressed_buttons(&self) -> GamepadButtonList {
        self.info.map(|i| i.pressed.clone()).unwrap_or_default()
    }
    /// Get all buttons released this frame
    #[inline]
    pub fn released_buttons(&self) -> GamepadButtonList {
        self.info.map(|i| i.released.clone()).unwrap_or_default()
    }
    /// Get the current state of all axes
    #[inline]
    pub fn axis_states(&self) -> GamepadAxisList {
        self.info.map(|i| i.axis.clone()).unwrap_or_default()
    }
    /// Get the vendor-specific name for a button
    #[inline]
    pub fn button_name(&self, btn: GamepadButton) -> &str {
        let vendor = self.vendor().unwrap_or(GamepadVendor::Unknown);
        btn.vendor_name(vendor)
    }
    /// Get the vendor-specific name for an axis
    #[inline]
    pub fn axis_name(&self, axis: GamepadAxis) -> &str {
        let vendor = self.vendor().unwrap_or(GamepadVendor::Unknown);
        axis.vendor_name(vendor)
    }
}

/// Iterator over all gamepad slots
pub struct GamepadsIter<'a> {
    enumerated: core::iter::Enumerate<slice::Iter<'a, Option<GamepadInfo>>>,
}

impl<'a> Iterator for GamepadsIter<'a> {
    type Item = GamepadState<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.enumerated.next().map(|(i, opt)| GamepadState {
            slot: GamepadSlot(i as u8),
            info: opt.as_ref(),
        })
    }
}

impl<'a> IntoIterator for &'a Gamepads {
    type Item = GamepadState<'a>;
    type IntoIter = GamepadsIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_slots()
    }
}

fn button_cast(btn: Button) -> GamepadButton {
    match btn {
        Button::South => GamepadButton::South,
        Button::East => GamepadButton::East,
        Button::North => GamepadButton::North,
        Button::West => GamepadButton::West,
        Button::LeftTrigger => GamepadButton::LeftShoulder,
        Button::LeftTrigger2 => GamepadButton::LeftTrigger,
        Button::RightTrigger => GamepadButton::RightShoulder,
        Button::RightTrigger2 => GamepadButton::RightTrigger,
        Button::Select => GamepadButton::Select,
        Button::Start => GamepadButton::Start,
        Button::Mode => GamepadButton::Menu,
        Button::LeftThumb => GamepadButton::LeftStick,
        Button::RightThumb => GamepadButton::RightStick,
        Button::DPadUp => GamepadButton::DPadUp,
        Button::DPadDown => GamepadButton::DPadDown,
        Button::DPadLeft => GamepadButton::DPadLeft,
        Button::DPadRight => GamepadButton::DPadRight,
        _ => GamepadButton::Unknown,
    }
}

fn axis_cast(axis: Axis) -> GamepadAxis {
    match axis {
        Axis::LeftStickX => GamepadAxis::LeftX,
        Axis::LeftStickY => GamepadAxis::LeftY,
        Axis::LeftZ => GamepadAxis::LeftTrigger,
        Axis::RightStickX => GamepadAxis::RightX,
        Axis::RightStickY => GamepadAxis::RightY,
        Axis::RightZ => GamepadAxis::RightTrigger,
        _ => GamepadAxis::Unknown,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_gamepads_default_empty() {
        let gamepads = Gamepads::default();
        assert!(!gamepads.is_connected(0u8));
        assert!(!gamepads.is_connected(GamepadSlot(0)));
    }

    #[test]
    fn test_gamepads_is_connected_empty() {
        let gamepads = Gamepads::default();
        for i in 0..16 {
            assert!(!gamepads.is_connected(i));
        }
    }

    #[test]
    fn test_gamepads_iter_slots_empty() {
        let gamepads = Gamepads::default();
        let slots: Vec<_> = gamepads.iter_slots().collect();
        assert_eq!(slots.len(), 16);

        for slot_state in slots {
            assert!(!slot_state.is_connected());
        }
    }

    #[test]
    fn test_gamepads_get_empty_slot() {
        let gamepads = Gamepads::default();
        let state = gamepads.get(0u8);

        assert!(!state.is_connected());
        assert_eq!(state.slot().0, 0);
        assert_eq!(state.name(), None);
        assert_eq!(state.uuid(), None);
        assert_eq!(state.vendor(), None);
    }

    #[test]
    fn test_gamepads_iter_empty() {
        let gamepads = Gamepads::default();
        let connected: Vec<_> = gamepads.iter().collect();
        assert_eq!(connected.len(), 0);
    }

    #[test]
    fn test_gamepads_into_iter() {
        let gamepads = Gamepads::default();
        let slots: Vec<_> = gamepads.into_iter().collect();
        assert_eq!(slots.len(), 16);
    }

    #[test]
    fn test_gamepad_state_disconnected() {
        let gamepads = Gamepads::default();
        let state = gamepads.get(0u8);

        assert!(!state.is_connected());
        assert_eq!(state.slot().0, 0);
    }

    #[test]
    fn test_gamepad_state_slot() {
        let gamepads = Gamepads::default();
        let state = gamepads.get(7u8);
        assert_eq!(state.slot().0, 7);
    }

    #[test]
    fn test_gamepad_state_button_states_disconnected() {
        let gamepads = Gamepads::default();
        let state = gamepads.get(0u8);

        assert!(!state.just_pressed(GamepadButton::North));
        assert!(!state.just_released(GamepadButton::South));
        assert!(!state.is_down(GamepadButton::West));
    }

    #[test]
    fn test_gamepad_state_axis_movement_disconnected() {
        let gamepads = Gamepads::default();
        let state = gamepads.get(0u8);

        assert_eq!(state.axis_movement(GamepadAxis::LeftX), 0.0);
        assert_eq!(state.axis_movement(GamepadAxis::RightY), 0.0);
        assert_eq!(state.axis_movement(GamepadAxis::LeftTrigger), 0.0);
    }

    #[test]
    fn test_gamepad_state_button_lists_disconnected() {
        let gamepads = Gamepads::default();
        let state = gamepads.get(0u8);

        assert!(state.down_buttons().is_empty());
        assert!(state.pressed_buttons().is_empty());
        assert!(state.released_buttons().is_empty());
    }

    #[test]
    fn test_gamepad_state_axis_states_disconnected() {
        let gamepads = Gamepads::default();
        let state = gamepads.get(0u8);

        assert_eq!(state.axis_states().strength(GamepadAxis::LeftX), 0.0);
        assert_eq!(state.axis_states().strength(GamepadAxis::RightY), 0.0);
    }

    #[test]
    fn test_gamepad_state_button_name_disconnected() {
        let gamepads = Gamepads::default();
        let state = gamepads.get(0u8);

        assert_eq!(state.button_name(GamepadButton::North), "Y");
        assert_eq!(state.button_name(GamepadButton::South), "A");
        assert_eq!(state.button_name(GamepadButton::Start), "Menu");
    }

    #[test]
    fn test_gamepad_state_axis_name_disconnected() {
        let gamepads = Gamepads::default();
        let state = gamepads.get(0u8);

        assert_eq!(state.axis_name(GamepadAxis::LeftX), "Left Stick X");
        assert_eq!(state.axis_name(GamepadAxis::RightY), "Right Stick Y");
        assert_eq!(state.axis_name(GamepadAxis::LeftTrigger), "LT");
    }
}
