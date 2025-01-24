use super::app::App;
use crate::ecs::schedules::{
    OnAudio, OnCleanup, OnEnginePostFrame, OnEnginePreFrame, OnFixedUpdate, OnPostFixedUpdate,
    OnPostFrame, OnPostRender, OnPostUpdate, OnPreFixedUpdate, OnPreFrame, OnPreRender,
    OnPreUpdate, OnRender, OnSetup, OnUpdate,
};
use bevy_ecs::prelude::Schedule;
use bevy_ecs::schedule::ExecutorKind;

pub trait Plugin {
    fn apply(self, app: App) -> App;
}

impl Plugin for OnFixedUpdate {
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
 ($app:expr, $( $schedule:expr ),* $(,)?) => {
        $(
            {
                let mut schedule = Schedule::new($schedule);
                schedule.set_executor_kind(ExecutorKind::SingleThreaded);
                $app.world.add_schedule(schedule);
                println!("added {:?}", $schedule);
            }
        )*
    };
}

pub struct BaseSchedules;
impl Plugin for BaseSchedules {
    fn apply(self, mut app: App) -> App {
        add_schedules!(
            app,
            OnSetup,
            OnEnginePreFrame,
            OnEnginePostFrame,
            OnPreFrame,
            OnPostFrame,
            OnPreUpdate,
            OnUpdate,
            OnPostUpdate,
            OnPreRender,
            OnRender,
            OnPostRender,
            OnAudio,
            OnCleanup,
        );
        app
    }
}
