use std::{any::TypeId, marker::PhantomData, sync::Arc};

use crate::{
    ecs::{app::App, prelude::*},
    prelude::PanicContext,
};
use bevy_ecs::{
    bundle::NoBundleEffect, schedule::ScheduleLabel, system::ScheduleSystem, world::SpawnBatchIter,
};
use rustc_hash::FxHashMap;

pub trait Screen2:
    Copy + Clone + Send + Sync + Eq + std::fmt::Debug + std::hash::Hash + 'static
{
    fn sys_set() -> ScreenSysSet<Self> {
        ScreenSysSet::default()
    }

    fn name() -> &'static str;
}

#[derive(SystemSet, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ScreenSysSet<T: Screen2>(PhantomData<T>);

impl<T: Screen2> Default for ScreenSysSet<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(Component, Clone, Copy)]
pub struct InScreen2<S: Screen2>(PhantomData<S>);

impl<S: Screen2> Default for InScreen2<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnExit2<S: Screen2>(PhantomData<S>);

impl<S: Screen2> Default for OnExit2<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnEnter2<S: Screen2>(PhantomData<S>);

impl<S: Screen2> Default for OnEnter2<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnEnterFrom {
    pub from: TypeId,
    pub to: TypeId,
}

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnExitTo {
    pub from: TypeId,
    pub to: TypeId,
}

#[inline(always)]
fn init_schedule_if_necessary<S: ScheduleLabel + Clone>(world: &mut World, label: S) {
    let need_init = world
        .try_schedule_scope(label.clone(), |_world, _schedule| {})
        .is_err();
    if need_init {
        let mut schedule = Schedule::new(label);
        schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        world.add_schedule(schedule);
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct ChangeScreenMsg2<S: Screen2>(PhantomData<S>);

#[derive(Clone)]
struct Screen2Data {
    name: &'static str,
    on_enter: Arc<dyn Fn(&mut World) + Send + Sync + 'static>,
    on_enter_from: Arc<dyn Fn(&mut World, TypeId) + Send + Sync + 'static>,
    on_exit: Arc<dyn Fn(&mut World) + Send + Sync + 'static>,
    on_exit_to: Arc<dyn Fn(&mut World, TypeId) + Send + Sync + 'static>,
}

#[derive(Resource, Default)]
pub struct Screens {
    ids: FxHashMap<TypeId, Screen2Data>,
    current: Option<TypeId>,
}

impl Screens {
    /// Returns true if the current screen is of type S
    #[inline]
    pub fn is_current<S: Screen2>(&self) -> bool {
        self.current == Some(TypeId::of::<S>())
    }

    /// Returns true if the screen of type S is registered
    #[inline]
    pub fn contains<S: Screen2>(&self) -> bool {
        self.ids.contains_key(&TypeId::of::<S>())
    }

    /// Returns the name of the current screen, if any
    #[inline]
    pub fn current_name(&self) -> Option<&'static str> {
        self.current
            .and_then(|id| self.ids.get(&id).map(|data| data.name))
    }

    #[inline]
    pub(crate) fn add_screen<S: Screen2>(&mut self) {
        self.ids.insert(
            TypeId::of::<S>(),
            Screen2Data {
                name: S::name(),
                on_enter: Arc::new(move |world: &mut World| {
                    log::debug!("Screen: on enter {}", S::name());
                    world.run_schedule(OnEnter2::<S>::default());
                }),
                on_enter_from: Arc::new(move |world: &mut World, from: TypeId| {
                    let to = TypeId::of::<S>();
                    let _ = world.try_run_schedule(OnEnterFrom { from, to });
                }),
                on_exit: Arc::new(move |world: &mut World| {
                    log::debug!("Screen: on exit {}", S::name());
                    world.run_schedule(OnExit2::<S>::default());
                }),
                on_exit_to: Arc::new(move |world: &mut World, to: TypeId| {
                    let from = TypeId::of::<S>();
                    let _ = world.try_run_schedule(OnExitTo { from, to });
                }),
            },
        );
    }

    #[inline]
    pub(crate) fn set_current<S: Screen2>(&mut self) {
        log::debug!("Screen: set current {}", S::name());
        self.current = Some(TypeId::of::<S>());
    }

    #[inline]
    pub(crate) fn clear_current(&mut self) {
        self.current = None;
    }
}

pub struct AppScreen<'w, S: Screen2> {
    pub world: &'w mut World,
    pub screen: PhantomData<S>,
}

impl<'w, S: Screen2> AppScreen<'w, S> {
    #[inline]
    #[track_caller]
    pub(crate) fn on_schedule<M>(
        &mut self,
        label: impl ScheduleLabel + Clone,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.world
            .try_schedule_scope(label.clone(), |_world, schedule| {
                schedule.add_systems(systems.in_set(S::sys_set()));
                schedule.configure_sets(S::sys_set().run_if(is_in_screen2::<S>));
            })
            .or_panic_with(move || format!("Failed to add systems to schedule: {label:?}"));

        self
    }

    #[inline]
    #[track_caller]
    pub fn on_enter<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnEnter2::<S>::default(), systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_enter_from<FS: Screen2, M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        let schedule = OnEnterFrom {
            from: TypeId::of::<FS>(),
            to: TypeId::of::<S>(),
        };
        init_schedule_if_necessary(self.world, schedule);
        self.on_schedule(schedule, systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_exit<M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.on_schedule(OnExit2::<S>::default(), systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_exit_to<TS: Screen2, M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        let schedule = OnExitTo {
            from: TypeId::of::<S>(),
            to: TypeId::of::<TS>(),
        };
        init_schedule_if_necessary(self.world, schedule);
        self.on_schedule(schedule, systems)
    }

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
}

#[inline(always)]
pub fn is_in_screen2<S: Screen2>(screens: Option<Res<Screens>>) -> bool {
    screens.is_some_and(|screens| screens.is_current::<S>())
}

#[inline(always)]
pub fn no_screen2_set(screens: Option<Res<Screens>>) -> bool {
    screens.is_none_or(|screens| screens.current.is_none())
}

pub trait Screen:
    Resource + std::fmt::Debug + std::hash::Hash + Clone + Eq + Send + 'static
{
    fn add_schedules(app: &mut App) -> &mut App {
        app
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct ChangeScreenEvt<S: Screen>(pub S);

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ScreenSchedule<SL: ScheduleLabel, S: Screen>(pub SL, pub S);

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnExit<S: Screen>(pub S);

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnEnter<S: Screen>(pub S);

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnChange<S: Screen> {
    pub from: S,
    pub to: S,
}

pub fn in_screen<S: Screen>(screen: S) -> impl FnMut(Option<Res<S>>) -> bool + Clone {
    move |s: Option<Res<S>>| s.is_some_and(|current| *current == screen)
}

#[derive(Clone, Copy)]
pub struct ChangeScreen<S: Screen>(pub S);

impl<S: Screen> Command for ChangeScreen<S> {
    fn apply(self, world: &mut World) {
        world.write_message(ChangeScreenEvt(self.0.clone()));
    }
}

pub(crate) fn change_screen_event_system<S: Screen>(world: &mut World) {
    world.resource_scope(|world, evt: Mut<Messages<ChangeScreenEvt<S>>>| {
        let mut cursor = evt.get_cursor();
        for evt in cursor.read(&evt) {
            let screen = evt.0.clone();
            if let Some(last_screen) = world.remove_resource::<S>() {
                log::debug!("Screen: OnExit({last_screen:?})");
                world.run_schedule(OnExit(last_screen.clone()));
                log::debug!("Screen: OnChange(from: {last_screen:?}, to: {screen:?})");
                world.run_schedule(OnChange {
                    from: last_screen,
                    to: screen.clone(),
                });
            }

            world.insert_resource(screen.clone());
            log::debug!("Screen: OnEnter({screen:?})");
            world.run_schedule(OnEnter(screen));
        }
    });
}

pub(crate) fn change_screen2_event_system<S: Screen2>(world: &mut World) {
    world.resource_scope(|world, evt: Mut<Messages<ChangeScreenMsg2<S>>>| {
        if evt.is_empty() {
            return;
        }

        let to_id = TypeId::of::<S>();
        let (from_id, from_actions, to_actions) = {
            let screens = world
                .get_resource::<Screens>()
                .or_panic("Screens resource not found");

            let from_id = screens.current;
            let from_actions = from_id.and_then(|id| screens.ids.get(&id)).cloned();
            let to_actions = screens
                .ids
                .get(&to_id)
                .or_panic("Target screen not registered")
                .clone();

            (from_id, from_actions, to_actions)
        };

        let mut cursor = evt.get_cursor();
        for _ in cursor.read(&evt) {
            if let Some(fa) = &from_actions {
                (fa.on_exit_to)(world, to_id);
                (fa.on_exit)(world);
            }

            world.resource_mut::<Screens>().set_current::<S>();

            (to_actions.on_enter)(world);
            if let Some(fid) = from_id {
                (to_actions.on_enter_from)(world, fid);
            }
        }
    });
}

pub(crate) fn clear_screen2_event_system(world: &mut World) {
    world.resource_scope(|world, evt: Mut<Messages<ClearScreenMsg2>>| {
        if evt.is_empty() {
            return;
        }

        let actions = {
            world.get_resource::<Screens>().and_then(|screens| {
                screens.current.map(|current| {
                    screens
                        .ids
                        .get(&current)
                        .or_panic("Current screen not found")
                        .clone()
                })
            })
        };

        if let Some(actions) = actions {
            (actions.on_exit)(world);
            world.resource_mut::<Screens>().clear_current();
        }
    });
}

#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct ClearScreenMsg2;

#[derive(Clone, Copy, Default)]
pub(crate) struct ChangeScreen2<S: Screen2>(pub PhantomData<S>);

impl<S: Screen2> Command for ChangeScreen2<S> {
    fn apply(self, world: &mut World) {
        world.write_message(ChangeScreenMsg2::<S>(PhantomData));
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct ClearScreen2;

impl Command for ClearScreen2 {
    fn apply(self, world: &mut World) {
        world.write_message(ClearScreenMsg2);
    }
}

pub trait Screen2CmdExt {
    fn set_screen<S: Screen2>(&mut self);
    fn clear_screen(&mut self);
}

impl Screen2CmdExt for Commands<'_, '_> {
    fn set_screen<S: Screen2>(&mut self) {
        self.queue(ChangeScreen2(PhantomData::<S>));
    }

    fn clear_screen(&mut self) {
        self.queue(ClearScreen2);
    }
}

pub struct Screen2World<'w, S: Screen2> {
    world: &'w mut World,
    _screen: PhantomData<S>,
}

impl<'w, S: Screen2> Screen2World<'w, S> {
    #[inline]
    #[track_caller]
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityWorldMut<'_> {
        self.world.spawn((InScreen2::<S>::default(), bundle))
    }

    #[inline]
    #[track_caller]
    pub fn spawn_empty(&mut self) -> EntityWorldMut<'_> {
        let mut e = self.world.spawn_empty();
        e.insert(InScreen2::<S>::default());
        e
    }

    #[inline]
    #[track_caller]
    pub fn spawn_batch<I>(&mut self, iter: I) -> impl Iterator<Item = Entity>
    where
        I: IntoIterator,
        I::Item: Bundle<Effect: NoBundleEffect>,
    {
        let batch = iter.into_iter().map(|b| (InScreen2::<S>::default(), b));
        self.world.spawn_batch(batch)
    }

    #[inline]
    #[track_caller]
    pub fn get_entity(&mut self, entity: Entity) -> Option<EntityRef<'_>> {
        let e = self.world.get_entity(entity).ok()?;
        e.contains::<InScreen2<S>>().then_some(e)
    }

    #[inline]
    #[track_caller]
    pub fn get_entity_mut(&mut self, entity: Entity) -> Option<EntityWorldMut<'_>> {
        let e = self.world.get_entity_mut(entity).ok()?;
        e.contains::<InScreen2<S>>().then_some(e)
    }

    #[inline]
    #[track_caller]
    pub fn attach(&mut self, entity: Entity) {
        let Some(mut e) = self.get_entity_mut(entity) else {
            return;
        };
        e.insert_if_new(InScreen2::<S>::default());
    }

    #[inline]
    #[track_caller]
    pub fn detach(&mut self, entity: Entity) {
        let Some(mut e) = self.get_entity_mut(entity) else {
            return;
        };
        e.remove::<InScreen2<S>>();
    }

    #[inline]
    #[track_caller]
    pub fn despawn_all(&mut self) {
        // TODO: I am not sure if we can avoid the allocation here
        let entities = self
            .world
            .query_filtered::<Entity, With<InScreen2<S>>>()
            .iter(self.world)
            .collect::<Vec<_>>();

        for e in entities {
            let _ = self.world.try_despawn(e);
        }
    }
}
