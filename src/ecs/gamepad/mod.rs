#![cfg(feature = "gamepad")]

mod data;
mod gamepads;

use bevy_ecs::{prelude::*, schedule::IntoScheduleConfigs};
pub use data::*;
pub use gamepads::*;

use super::{app::App, input::InputSysSet, plugin::Plugin, schedules::OnEnginePreFrame};

#[derive(Default)]
pub struct GamepadPlugin;

impl Plugin for GamepadPlugin {
    fn apply(&self, app: &mut App) {
            app.on_setup(setup_system)
            .on_schedule(
                OnEnginePreFrame,
                sync_gilrs_events_system.in_set(InputSysSet),
            )
            .configure_sets(OnEnginePreFrame, InputSysSet);
    }
}

fn setup_system(world: &mut World) {
        let raw = match RawGilrs::new() {
            Ok(raw) => raw,
            Err(e) => {
                log::error!("Failed to initialize gamepad system: {}", e);
                return;
            }
        };
        
        world.insert_non_send_resource(raw);
            world.insert_resource(Gamepads::default());
}
