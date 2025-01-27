use super::prelude::*;
use crate::ecs::plugin::{BaseSchedules, Plugin};
use crate::ecs::schedules::{
    OnAudio, OnCleanup, OnEnginePostFrame, OnEnginePreFrame, OnFixedUpdate, OnPostFixedUpdate,
    OnPostFrame, OnPostRender, OnPostUpdate, OnPreFixedUpdate, OnPreFrame, OnPreRender,
    OnPreUpdate, OnRender, OnSetup, OnUpdate,
};
use crate::ecs::screen::{in_screen, Screen};
use bevy_ecs::schedule::ScheduleLabel;
use bevy_tasks::{ComputeTaskPool, TaskPool};
use corelib::app::{LogConfig, WindowConfig};

pub struct App {
    pub world: World,
    pub(crate) fixed_updates: Vec<u8>,
    pub(crate) window_config: WindowConfig,
    pub(crate) log_config: LogConfig,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let world = World::new();

        let app = Self {
            world,
            fixed_updates: vec![],
            window_config: Default::default(),
            log_config: Default::default(),
        };

        ComputeTaskPool::get_or_init(TaskPool::default);
        app.add_plugin(BaseSchedules)
    }

    #[inline]

    pub(crate) fn with_window(mut self, config: WindowConfig) -> Self {
        self.window_config = config;
        self
    }

    #[inline]

    pub(crate) fn with_log(mut self, config: LogConfig) -> Self {
        self.log_config = config;
        self
    }

    #[inline]
    pub fn with_screen<S: Screen>(mut self, screen: S) -> Self {
        S::add_schedules(self).add_systems(OnEngineSetup, move |mut cmds: Commands| {
            cmds.queue(ChangeScreen(screen.clone()))
        })
    }

    #[inline]
    pub fn add_plugin(self, config: impl Plugin) -> Self {
        config.apply(self)
    }

    #[inline]

    pub fn add_screen_systems<S: Screen, M>(
        self,
        screen: S,
        label: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
    ) -> Self {
        self.add_systems(label, (systems).run_if(in_screen(screen)))
    }

    #[inline]

    pub fn add_systems<M>(
        mut self,
        label: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
    ) -> Self {
        self.world
            .try_schedule_scope(label, |_world, schedule| {
                schedule.add_systems(systems);
            })
            .unwrap();
        self
    }

    #[track_caller]
    pub fn configure_sets(
       mut self,
        label: impl ScheduleLabel,
        sets: impl IntoSystemSetConfigs,
    ) -> Self {
        self.world
            .try_schedule_scope(label, move |_world, schedule| {
                schedule.configure_sets(sets);
            })
            .unwrap();
        self
    }

    #[inline]
    #[track_caller]
    pub fn insert_resource<R: Resource>(&mut self, value: R) {
        self.world.insert_resource(value);
    }

    pub fn run(self) -> Result<(), String> {
        let Self {
            mut world,
            fixed_updates,
            window_config,
            log_config,
        } = self;
        let mut builder = crate::init_with(|| {
            world.run_schedule(OnEngineSetup);
            world.run_schedule(OnSetup);
            world
        });

        builder = builder.with_window(window_config).with_logs(log_config);

        builder = builder.pre_update(|world: &mut World| {
            world.run_schedule(OnEnginePreFrame);
            world.run_schedule(OnPreFrame);
        });

        for fps in fixed_updates {
            builder = builder.fixed_update(1.0 / (fps as f32), move |world: &mut World| {
                world.run_schedule(OnPreFixedUpdate(fps));
                world.run_schedule(OnFixedUpdate(fps));
                world.run_schedule(OnPostFixedUpdate(fps));
            });
        }

        builder = builder.update(|world: &mut World| {
            world.run_schedule(OnPreUpdate);
            world.run_schedule(OnUpdate);
            world.run_schedule(OnPostUpdate);

            world.run_schedule(OnPreRender);
            world.run_schedule(OnRender);
            world.run_schedule(OnPostRender);

            world.run_schedule(OnAudio);

            world.run_schedule(OnPostFrame);
            world.run_schedule(OnEnginePostFrame);
        });

        builder = builder.cleanup(|world: &mut World| world.run_schedule(OnCleanup));

        builder.run()
    }
}
