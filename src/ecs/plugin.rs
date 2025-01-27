use super::app::App;
use crate::ecs::schedules::{
    OnAudio, OnCleanup, OnEnginePostFrame, OnEnginePreFrame, OnFixedUpdate, OnPostFixedUpdate,
    OnPostFrame, OnPostRender, OnPostUpdate, OnPreFixedUpdate, OnPreFrame, OnPreRender,
    OnPreUpdate, OnRender, OnSetup, OnUpdate,
};
use crate::prelude::OnEngineSetup;
use bevy_ecs::prelude::Schedule;
use bevy_ecs::schedule::ExecutorKind;

pub trait Plugin {
    fn apply(self, app: App) -> App;
}

pub struct FixedUpdate(pub u8);
impl Plugin for FixedUpdate {
    fn apply(self, mut app: App) -> App {
        let fps = self.0;

        if app.fixed_updates.contains(&fps) {
            log::warn!("Ignoring FixedUpdate({fps}) because it's already registered");
            return app;
        }

        app.world.add_schedule(Schedule::new(OnPreFixedUpdate(fps)));
        app.world.add_schedule(Schedule::new(OnFixedUpdate(fps)));
        app.world
            .add_schedule(Schedule::new(OnPostFixedUpdate(fps)));

        app.fixed_updates.push(fps);
        app
    }
}

macro_rules! add_schedules {
    ($app:expr, $( $schedule:ident : $multi_threaded:expr ),* $(,)?) => {
        $(
            {
                let mut schedule = Schedule::new($schedule);
                if !$multi_threaded {
                    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
                }
                $app.world.add_schedule(schedule);
                // FIXME: this is not printer because logs are not initialized yet...
                log::debug!("Added schedule {:?}, multithread: {:?}", stringify!($schedule), $multi_threaded);
            }
        )*
    };
}

pub struct BaseSchedules;
impl Plugin for BaseSchedules {
    fn apply(self, mut app: App) -> App {
        add_schedules!(
            app,
            OnEngineSetup: false,
            OnEnginePreFrame: false,
            OnEnginePostFrame: false,
            OnSetup: false,
            OnPreFrame: true,
            OnPostFrame: true,
            OnPreUpdate: true,
            OnUpdate: true,
            OnPostUpdate: true,
            OnPreRender: true,
            OnRender: false,
            OnPostRender: false,
            OnAudio: false,
            OnCleanup: false,
        );
        app
    }
}
