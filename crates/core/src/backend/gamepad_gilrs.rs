#![cfg(feature = "gamepad")]

use crate::input::{GamepadAxis, GamepadButton, GamepadState};
use atomic_refcell::AtomicRefCell;
use crossbeam_channel::{unbounded, Receiver, Sender};
use gilrs::{Axis, Button, Event, EventType, GamepadId, Gilrs};
use heapless::FnvIndexMap;
use once_cell::sync::Lazy;
use smallvec::SmallVec;
use std::sync::Mutex;

use crate::input::GAMEPADS_CONNECTED_POT2;

// GilRS is not send sync, it cannot be part of a AtomicRef, so I am sending events to a channel that's
// reading the events before the game executes the update callback
fn init_gilrs() -> Receiver<Event> {
    let (tx, rx): (Sender<Event>, Receiver<Event>) = unbounded();

    // TODO This will panic in wasm32, we have two options, full wasm32 backend or add a new code for wasm32 platform using gilrs
    std::thread::spawn(move || {
        let mut gilrs = Gilrs::new().expect("Failed to initialize gilrs");
        loop {
            while let Some(event) = gilrs.next_event_blocking(None) {
                let res = tx.send(event);
                if let Err(err) = res {
                    log::error!("Error sending GilRS event: '{}'", err.to_string());
                }
            }
            gilrs.inc();
        }
    });
    rx
}

pub(crate) struct GilrsBackend {
    rx: Receiver<Event>,

    pub(crate) state: GamepadState,

    ids: FnvIndexMap<GamepadId, usize, GAMEPADS_CONNECTED_POT2>,
    id_count: usize,
}

impl Default for GilrsBackend {
    fn default() -> Self {
        let rx = init_gilrs();

        Self {
            rx,
            state: Default::default(),
            ids: Default::default(),
            id_count: 0,
        }
    }
}

impl GilrsBackend {
    pub fn tick(&mut self) {
        self.state.tick();

        while let Ok(Event { id, event, .. }) = self.rx.try_recv() {
            match event {
                EventType::ButtonPressed(btn, _) => {
                    debug_assert!(
                        self.ids.get(&id).is_some(),
                        "Gamepad '{}' not registered?",
                        id
                    );
                    let gp = self.ids.get(&id).and_then(|&id| self.state.get_mut(id));

                    if let Some(gp) = gp {
                        gp.press(button_cast(btn));
                    }
                }
                EventType::ButtonRepeated(_, _) => {}
                EventType::ButtonReleased(btn, _) => {
                    debug_assert!(
                        self.ids.get(&id).is_some(),
                        "Gamepad '{}' not registered?",
                        id
                    );
                    let gp = self.ids.get(&id).and_then(|&id| self.state.get_mut(id));

                    if let Some(gp) = gp {
                        gp.release(button_cast(btn));
                    }
                }
                EventType::ButtonChanged(_, _, _) => {}
                EventType::AxisChanged(axis, strength, _) => {
                    debug_assert!(
                        self.ids.get(&id).is_some(),
                        "Gamepad '{}' not registered?",
                        id
                    );
                    let gp = self.ids.get(&id).and_then(|&id| self.state.get_mut(id));

                    if let Some(gp) = gp {
                        gp.set_axis_strength(axis_cast(axis), strength);
                    }
                }
                EventType::Connected => {
                    // Reuse ID if exists or assign a new one if not
                    let current_id = self.ids.get(&id).cloned().unwrap_or_else(|| {
                        let next_id = self.id_count;
                        self.id_count += 1;
                        next_id
                    });
                    self.state.add(current_id);
                }
                EventType::Disconnected => {
                    debug_assert!(
                        self.ids.get(&id).is_some(),
                        "Gamepad '{}' not registered?",
                        id
                    );
                    if let Some(&id) = self.ids.get(&id) {
                        self.state.remove(id);
                    }
                }
                EventType::Dropped => {}
                EventType::ForceFeedbackEffectCompleted => {}
                _ => {}
            }
        }
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
