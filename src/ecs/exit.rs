use crate::app::close_window;
use bevy_ecs::prelude::*;

#[derive(Debug, Clone, Copy, Message)]
pub(super) struct AppExitEvt;

pub(super) fn app_exit_system(evt: MessageReader<AppExitEvt>) {
    if !evt.is_empty() {
        log::info!("Closing app...");
        close_window();
    }
}

pub struct AppExitCommand;

impl Command for AppExitCommand {
    fn apply(self, world: &mut World) {
        world.send_event(AppExitEvt);
    }
}

pub trait ExitCmdExt {
    fn exit(&mut self);
}

impl ExitCmdExt for Commands<'_, '_> {
    fn exit(&mut self) {
        self.queue(AppExitCommand);
    }
}
