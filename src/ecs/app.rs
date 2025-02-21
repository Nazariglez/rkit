use super::exit::app_exit_system;
use super::prelude::*;
use crate::app::{LogConfig, WindowConfig};
use crate::ecs::plugin::{BaseSchedules, Plugin};
use crate::ecs::schedules::{
    OnCleanup, OnEnginePostFrame, OnEnginePreFrame, OnFixedUpdate, OnPostFixedUpdate, OnPostFrame,
    OnPostRender, OnPostUpdate, OnPreFixedUpdate, OnPreFrame, OnPreRender, OnPreUpdate, OnRender,
    OnSetup, OnUpdate,
};
use crate::ecs::screen::Screen;
use bevy_ecs::event::EventRegistry;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::system::SystemId;
use bevy_tasks::{ComputeTaskPool, TaskPool};

pub struct App {
    pub world: World,
    exit_sys: SystemId,
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
        let mut world = World::new();

        let exit_sys = world.register_system(app_exit_system);
        let app = Self {
            world,
            exit_sys,
            fixed_updates: vec![],
            window_config: Default::default(),
            log_config: Default::default(),
        };

        ComputeTaskPool::get_or_init(TaskPool::default);
        app.add_plugin(BaseSchedules)
            .add_event::<AppExitEvt>()
            .add_systems(OnEnginePreFrame, bevy_ecs::event::event_update_system)
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
    pub fn with_screen<S: Screen>(self, screen: S) -> Self {
        S::add_schedules(self).add_systems(OnEngineSetup, move |mut cmds: Commands| {
            cmds.queue(ChangeScreen(screen.clone()))
        })
    }

    #[inline]
    pub fn add_plugin(self, config: impl Plugin) -> Self {
        config.apply(self)
    }

    #[inline]
    #[track_caller]
    pub fn add_screen_systems<SL: ScheduleLabel + Eq + Clone + std::hash::Hash, S: Screen, M>(
        mut self,
        screen: S,
        label: SL,
        systems: impl IntoSystemConfigs<M>,
    ) -> Self {
        let schedule_id = ScreenSchedule(label.clone(), screen.clone());

        let needs_initialization = self
            .world
            .try_schedule_scope(schedule_id.clone(), |_world, _sch| {})
            .is_err();

        if needs_initialization {
            let executor = self
                .world
                .schedule_scope(label.clone(), |_w, sch| sch.get_executor_kind());

            let mut schedule = Schedule::new(schedule_id.clone());
            schedule.set_executor_kind(executor);
            self.world.add_schedule(schedule);

            let sc_id_clone = schedule_id.clone();
            self.world.schedule_scope(label, move |_world, sc| {
                let sys = move |world: &mut World| {
                    let is_in_screen = world
                        .get_resource::<S>()
                        .is_some_and(|s| *s == screen.clone());

                    if is_in_screen {
                        world.run_schedule(sc_id_clone.clone());
                    }
                };
                sc.add_systems(sys);
            });
        }

        self.add_systems(schedule_id, systems)
    }

    #[inline]
    #[track_caller]
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

    #[inline]
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
    pub fn configure_screen_sets(
        self,
        screen: impl Screen,
        label: impl ScheduleLabel + Clone + Eq + std::hash::Hash,
        sets: impl IntoSystemSetConfigs,
    ) -> Self {
        self.configure_sets(ScreenSchedule(label, screen), sets)
    }

    #[inline]
    #[track_caller]
    pub fn add_event<E: Event>(mut self) -> Self {
        if !self.world.contains_resource::<Events<E>>() {
            EventRegistry::register_event::<E>(&mut self.world);
        }
        self
    }

    #[inline]
    #[track_caller]
    pub fn add_resource<R: Resource>(mut self, value: R) -> Self {
        self.world.insert_resource(value);
        self
    }

    pub fn run(self) -> Result<(), String> {
        let Self {
            mut world,
            exit_sys,
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

        builder = builder.update(move |world: &mut World| {
            world.run_schedule(OnPreUpdate);
            world.run_schedule(OnUpdate);
            world.run_schedule(OnPostUpdate);

            world.run_schedule(OnPreRender);
            world.run_schedule(OnRender);
            world.run_schedule(OnPostRender);

            world.run_schedule(OnPostFrame);
            world.run_schedule(OnEnginePostFrame);

            world.clear_trackers();

            world.run_system(exit_sys).unwrap();
        });

        builder = builder.cleanup(|world: &mut World| world.run_schedule(OnCleanup));

        builder.run()
    }
}
