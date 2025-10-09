use std::marker::PhantomData;

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
use bevy_ecs::{
    message::MessageRegistry,
    schedule::{InternedSystemSet, ScheduleLabel},
    system::{IntoObserverSystem, ScheduleSystem, SystemId},
};
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
            .add_message::<AppExitEvt>()
            .on_schedule(OnEnginePreFrame, bevy_ecs::message::message_update_system);

        app.add_message::<ClearScreenMsg2>().on_schedule(
            OnEnginePreFrame,
            clear_screen2_event_system.after(bevy_ecs::message::message_update_system),
        );

        app
    }

    #[inline]
    pub(crate) fn add_window(&mut self, config: WindowConfig) -> &mut Self {
        self.window_config = config;
        self
    }

    #[inline]
    pub(crate) fn add_log(&mut self, config: LogConfig) -> &mut Self {
        self.log_config = config;
        self
    }

    #[inline]
    pub fn add_screen<S: Screen>(&mut self, screen: S) -> &mut Self {
        self.add_message::<ChangeScreenEvt<S>>().on_schedule(
            OnEnginePreFrame,
            change_screen_event_system::<S>.after(bevy_ecs::message::message_update_system),
        );

        S::add_schedules(self).on_schedule(OnEngineSetup, move |mut cmds: Commands| {
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
    pub fn add_message<M: Message>(&mut self) -> &mut Self {
        if !self.world.contains_resource::<Messages<M>>() {
            MessageRegistry::register_message::<M>(&mut self.world);
        }
        self
    }

    #[inline]
    pub fn on_event<E: Event, B: Bundle, M>(
        &mut self,
        system: impl IntoObserverSystem<E, B, M>,
    ) -> &mut Self {
        self.world.add_observer(system);
        self
    }

    #[inline]
    #[track_caller]
    pub fn on_schedule<M>(
        &mut self,
        label: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
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
    pub fn configure_sets<M>(
        &mut self,
        label: impl ScheduleLabel,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
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
    pub fn on_screen_schedule<SL: ScheduleLabel + Eq + Clone + std::hash::Hash, S: Screen, M>(
        &mut self,
        screen: S,
        label: SL,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
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

        self.on_schedule(schedule_id, systems)
    }

    #[inline]
    #[track_caller]
    pub fn configure_screen_sets<M>(
        &mut self,
        screen: impl Screen,
        label: impl ScheduleLabel + Clone + Eq + std::hash::Hash,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(ScreenSchedule(label, screen), sets)
    }

    #[inline]
    #[track_caller]
    pub fn insert_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.world.insert_resource(value);
        self
    }

    #[inline]
    #[track_caller]
    pub fn insert_non_send_resource<R: 'static>(&mut self, value: R) -> &mut Self {
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

// Schedule management methods
impl App {
    #[inline]
    #[track_caller]
    pub fn on_setup<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnSetup, systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_update<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnUpdate, systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_pre_frame<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnPreFrame, systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_post_frame<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnPostFrame, systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_pre_update<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnPreUpdate, systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_post_update<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnPostUpdate, systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_render<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnRender, systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_pre_fixed_update<M>(
        &mut self,
        fps: u8,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnPreFixedUpdate(fps), systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_post_fixed_update<M>(
        &mut self,
        fps: u8,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnPostFixedUpdate(fps), systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_fixed_update<M>(
        &mut self,
        fps: u8,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnFixedUpdate(fps), systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_pre_render<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnPreRender, systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_post_render<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnPostRender, systems)
    }

    #[inline]
    #[track_caller]
    pub fn config_setup_sets<M>(
        &mut self,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnSetup, sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_update_sets<M>(
        &mut self,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnUpdate, sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_pre_frame_sets<M>(
        &mut self,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnPreFrame, sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_post_frame_sets<M>(
        &mut self,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnPostFrame, sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_pre_update_sets<M>(
        &mut self,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnPreUpdate, sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_post_update_sets<M>(
        &mut self,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnPostUpdate, sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_render_sets<M>(
        &mut self,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnRender, sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_pre_fixed_update_sets<M>(
        &mut self,
        fps: u8,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnPreFixedUpdate(fps), sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_post_fixed_update_sets<M>(
        &mut self,
        fps: u8,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnPostFixedUpdate(fps), sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_fixed_update_sets<M>(
        &mut self,
        fps: u8,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnFixedUpdate(fps), sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_pre_render_sets<M>(
        &mut self,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnPreRender, sets)
    }

    #[inline]
    #[track_caller]
    pub fn config_post_render_sets<M>(
        &mut self,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        self.configure_sets(OnPostRender, sets)
    }

    #[inline]
    #[track_caller]
    pub fn add_screen2<S: Screen2>(&mut self, cb: impl FnOnce(&mut AppScreen<S>)) -> &mut Self {
        let mut screens = self.world.remove_resource::<Screens>().unwrap_or_default();
        let is_initiated = screens.contains::<S>();
        if !is_initiated {
            screens.add_screen::<S>();

            let mut schedule = Schedule::new(OnEnter2::<S>::default());
            schedule.set_executor_kind(ExecutorKind::SingleThreaded);
            self.world.add_schedule(schedule);

            let mut schedule = Schedule::new(OnExit2::<S>::default());
            schedule.set_executor_kind(ExecutorKind::SingleThreaded);
            self.world.add_schedule(schedule);

            self.add_message::<ChangeScreenMsg2<S>>().on_schedule(
                OnEnginePreFrame,
                change_screen2_event_system::<S>.after(bevy_ecs::message::message_update_system),
            );
        }
        self.world.insert_resource(screens);

        cb(&mut AppScreen {
            world: &mut self.world,
            screen: PhantomData,
        });
        self
    }
}
