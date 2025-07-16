use super::prelude::*;
use crate::{
    app::{LogConfig, WindowConfig},
    ecs::{
        exit::app_exit_system,
        plugin::{BaseSchedules, Plugin},
        schedules::{
            OnCleanup, OnEnginePostFrame, OnEnginePreFrame, OnFixedUpdate, OnPostFixedUpdate,
            OnPostFrame, OnPostRender, OnPostUpdate, OnPreFixedUpdate, OnPreFrame, OnPreRender,
            OnPreUpdate, OnRender, OnSetup, OnUpdate,
        },
        screen::Screen,
    },
};
use bevy_ecs::{event::EventRegistry, schedule::ScheduleLabel, system::SystemId};
use bevy_tasks::{ComputeTaskPool, TaskPool};

pub(crate) type AppBuilder = corelib::AppBuilder<World>;

pub struct App {
    pub world: World,

    pub(crate) fixed_updates: Vec<u8>,
    pub(crate) window_config: WindowConfig,
    pub(crate) log_config: LogConfig,

    exit_sys: SystemId,

    extensions: Vec<Box<dyn FnOnce(AppBuilder) -> AppBuilder>>,
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
        let mut app = Self {
            world,
            exit_sys,
            fixed_updates: vec![],
            window_config: Default::default(),
            log_config: Default::default(),
            extensions: vec![],
        };

        ComputeTaskPool::get_or_init(TaskPool::default);
        app.add_plugin(BaseSchedules)
            .add_event::<AppExitEvt>()
            .add_systems(OnEnginePreFrame, bevy_ecs::event::event_update_system);

        app
    }

    #[inline]
    pub(crate) fn with_window(&mut self, config: WindowConfig) -> &mut Self {
        self.window_config = config;
        self
    }

    #[inline]
    pub(crate) fn with_log(&mut self, config: LogConfig) -> &mut Self {
        self.log_config = config;
        self
    }

    #[inline]
    pub fn with_screen<S: Screen>(&mut self, screen: S) -> &mut Self {
        self.add_event::<ChangeScreenEvt<S>>().add_systems(
            OnEnginePreFrame,
            change_screen_event_system::<S>.after(bevy_ecs::event::event_update_system),
        );

        S::add_schedules(self).add_systems(OnEngineSetup, move |mut cmds: Commands| {
            cmds.queue(ChangeScreen(screen.clone()))
        })
    }

    #[inline]
    pub fn add_plugin(&mut self, config: impl Plugin) -> &mut Self {
        config.apply(self);
        self
    }

    #[inline]
    #[track_caller]
    pub fn add_screen_systems<SL: ScheduleLabel + Eq + Clone + std::hash::Hash, S: Screen, M>(
        &mut self,
        screen: S,
        label: SL,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
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
        &mut self,
        label: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
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
        &mut self,
        label: impl ScheduleLabel,
        sets: impl IntoSystemSetConfigs,
    ) -> &mut Self {
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
        &mut self,
        screen: impl Screen,
        label: impl ScheduleLabel + Clone + Eq + std::hash::Hash,
        sets: impl IntoSystemSetConfigs,
    ) -> &mut Self {
        self.configure_sets(ScreenSchedule(label, screen), sets)
    }

    #[inline]
    #[track_caller]
    pub fn add_event<E: Event>(&mut self) -> &mut Self {
        if !self.world.contains_resource::<Events<E>>() {
            EventRegistry::register_event::<E>(&mut self.world);
        }
        self
    }

    #[inline]
    #[track_caller]
    pub fn add_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.world.insert_resource(value);
        self
    }

    #[inline]
    #[track_caller]
    pub fn add_non_send_resource<R: 'static>(&mut self, value: R) -> &mut Self {
        self.world.insert_non_send_resource(value);
        self
    }

    pub(crate) fn extend_with<F>(&mut self, cb: F) -> &mut Self
    where
        F: FnOnce(AppBuilder) -> AppBuilder + 'static,
    {
        self.extensions.push(Box::new(cb));
        self
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut world = std::mem::take(&mut self.world);
        let exit_sys = self.exit_sys;
        let window_config = self.window_config.clone();
        let log_config = self.log_config.clone();
        let extensions = std::mem::take(&mut self.extensions);

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

        for fps in self.fixed_updates.iter().cloned() {
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

        for ext in extensions {
            builder = ext(builder);
        }

        builder.run()
    }
}
