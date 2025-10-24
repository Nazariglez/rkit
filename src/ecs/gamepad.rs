#![cfg(feature = "gamepad")]

use super::{app::App, bevy_ecs::prelude::*, plugin::Plugin, schedules::OnEnginePreFrame};
use core::slice;
use gilrs::{Axis, Button, Event, EventType, GamepadId as GilrsGamepadId, Gilrs};
use heapless::index_map::FnvIndexMap;
use uuid::Uuid;

// Passing this env variable we can control the size of the hashset to reduce memory consume.
// 16 gamepads at once seems more than enough, most games have a max of 4-8, and other libs as
// SDL seems to allow a max of 16.
const MAX_GAMEPADS: usize = 16;

#[derive(Default)]
pub struct GamepadPlugin;

impl Plugin for GamepadPlugin {
    fn apply(&self, app: &mut App) {
        let raw = match RawGilrs::new() {
            Ok(raw) => raw,
            Err(e) => {
                log::error!("Failed to initialize gamepad system: {}", e);
                return;
            }
        };

        app.insert_non_send_resource(raw)
            .insert_resource(Gamepads::default())
            .on_schedule(OnEnginePreFrame, sync_gilrs_events_system);
    }
}

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

struct GamepadInfo {
    id: GilrsGamepadId,
    name: String,
    uuid: Uuid,
}

#[derive(Default, Resource)]
pub struct Gamepads {
    slots: [Option<GamepadInfo>; MAX_GAMEPADS],
    ids: FnvIndexMap<GilrsGamepadId, GamepadSlot, MAX_GAMEPADS>,
}

impl Gamepads {
    pub fn is_connected(&self, slot: impl Into<GamepadSlot>) -> bool {
        let slot: GamepadSlot = slot.into();
        self.slots[slot.0 as usize].is_some()
    }

    fn add_gamepad(&mut self, id: GilrsGamepadId, name: &str, uuid: Uuid) -> Option<GamepadSlot> {
        let slot_idx = self.slots.iter().position(|slot| slot.is_none())?;
        self.slots[slot_idx] = Some(GamepadInfo {
            id,
            name: name.to_string(),
            uuid,
        });

        log::info!("Gamepad '{name}' connected on slot {slot_idx}");
        let slot = GamepadSlot(slot_idx as u8);
        self.ids.insert(id, slot).unwrap();
        Some(slot)
    }

    fn remove_gamepad(&mut self, id: GilrsGamepadId) -> Option<(GamepadSlot, GamepadInfo)> {
        self.ids.remove(&id).and_then(|slot| {
            let info = self.slots[slot.0 as usize].take();
            debug_assert!(info.is_some());
            log::info!(
                "Gamepad '{}' disconnected from slot {}",
                info.as_ref().map_or("-unknown-", |info| &info.name),
                slot.0
            );
            Some((slot, info.unwrap()))
        })
    }
}

struct RawGilrs(Gilrs);

impl RawGilrs {
    pub fn new() -> Result<Self, String> {
        let gilrs = Gilrs::new().map_err(|e| e.to_string())?;
        Ok(Self(gilrs))
    }
}

fn sync_gilrs_events_system(mut gamepads: ResMut<Gamepads>, mut raw: NonSendMut<RawGilrs>) {
    while let Some(Event { id, event, .. }) = raw.0.next_event() {
        match event {
            EventType::ButtonPressed(button, code) => {}
            EventType::ButtonRepeated(button, code) => {}
            EventType::ButtonReleased(button, code) => {}
            EventType::ButtonChanged(button, _, code) => {}
            EventType::AxisChanged(axis, _, code) => {}
            EventType::Connected => {
                let info = raw.0.gamepad(id);
                let uuid = Uuid::from_bytes(info.uuid());
                let slot = gamepads.add_gamepad(id, info.name(), uuid);
                match slot {
                    Some(slot) => {
                        log::debug!(
                            "Gamepad connected '{:?}': raw_id={:?}, name={}, uuid={}",
                            slot,
                            id,
                            info.name(),
                            uuid,
                        );
                    }
                    None => {
                        log::warn!(
                            "Gamepad connection ignored, not enoguh slots. raw_id='{:?}', name='{}', uuid='{:?}'",
                            info.id(),
                            info.name(),
                            uuid
                        );
                    }
                }
            }
            EventType::Disconnected => {
                let info = gamepads.remove_gamepad(id);
                match info {
                    Some((slot, info)) => {
                        log::debug!(
                            "Gamepad disconnected '{slot:?}': raw_id={:?}, name='{}', uuid='{:?}'",
                            id,
                            info.name,
                            info.uuid
                        );
                    }
                    None => {
                        let info = raw.0.gamepad(id);
                        log::warn!(
                            "Gamepad disconnection ignored, not found. raw_id='{:?}', name='{}', uuid='{:?}'",
                            id,
                            info.name(),
                            info.uuid(),
                        );
                    }
                }
            }
            EventType::Dropped => {}
            EventType::ForceFeedbackEffectCompleted => {}
            _ => {}
        }

        raw.0.inc();
    }
}

#[derive(Clone, Copy)]
pub struct GamepadState<'a> {
    slot: GamepadSlot,
    info: Option<&'a GamepadInfo>,
}

impl<'a> GamepadState<'a> {
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.info.is_some()
    }
    #[inline]
    pub fn slot(&self) -> GamepadSlot {
        self.slot
    }
    #[inline]
    pub fn name(&self) -> Option<&'a str> {
        self.info.map(|i| i.name.as_str())
    }
    #[inline]
    pub fn uuid(&self) -> Option<&'a Uuid> {
        self.info.map(|i| &i.uuid)
    }
}

/// Iterate all slots (connected or not).
pub struct GamepadsIter<'a> {
    enumerated: core::iter::Enumerate<slice::Iter<'a, Option<GamepadInfo>>>,
}

impl Gamepads {
    #[inline]
    pub fn iter(&self) -> GamepadsIter<'_> {
        GamepadsIter {
            enumerated: self.slots.iter().enumerate(),
        }
    }

    /// Iterate only connected pads.
    #[inline]
    pub fn iter_connected(&self) -> impl Iterator<Item = GamepadState<'_>> {
        self.slots.iter().enumerate().filter_map(|(i, opt)| {
            opt.as_ref().map(|info| GamepadState {
                slot: GamepadSlot(i as u8),
                info: Some(info),
            })
        })
    }

    /// Get a single slot view.
    #[inline]
    pub fn gamepad(&self, slot: impl Into<GamepadSlot>) -> GamepadState<'_> {
        let s = slot.into();
        GamepadState {
            slot: s,
            info: self.slots[s.0 as usize].as_ref(),
        }
    }
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

/// Sugar: `for state in &gamepads { ... }`
impl<'a> IntoIterator for &'a Gamepads {
    type Item = GamepadState<'a>;
    type IntoIter = GamepadsIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
