use std::{any::TypeId, marker::PhantomData, sync::Arc};

use crate::{ecs::prelude::*, prelude::PanicContext};
use bevy_ecs::{bundle::NoBundleEffect, schedule::ScheduleLabel, system::ScheduleSystem};
use rustc_hash::FxHashMap;

pub trait Screen:
    Copy + Clone + Send + Sync + Eq + std::fmt::Debug + std::hash::Hash + 'static
{
    fn sys_set() -> ScreenSysSet<Self> {
        ScreenSysSet::default()
    }

    fn name() -> &'static str;
}

#[derive(SystemSet, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ScreenSysSet<T: Screen>(PhantomData<T>);

impl<T: Screen> Default for ScreenSysSet<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(Component, Clone, Copy)]
pub struct InScreen<S: Screen>(PhantomData<S>);

impl<S: Screen> Default for InScreen<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct OnExit<S: Screen>(PhantomData<S>);

impl<S: Screen> Default for OnExit<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct OnEnter<S: Screen>(PhantomData<S>);

impl<S: Screen> Default for OnEnter<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct OnEnterFrom {
    pub from: TypeId,
    pub to: TypeId,
}

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct OnExitTo {
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
pub(crate) struct ChangeScreenMsg<S: Screen>(PhantomData<S>);

#[derive(Clone)]
struct ScreenData {
    name: &'static str,
    destroy_on_exit: bool,
    on_enter: Arc<dyn Fn(&mut World) + Send + Sync + 'static>,
    on_enter_from: Arc<dyn Fn(&mut World, TypeId) + Send + Sync + 'static>,
    on_exit: Arc<dyn Fn(&mut World) + Send + Sync + 'static>,
    on_exit_to: Arc<dyn Fn(&mut World, TypeId) + Send + Sync + 'static>,
}

#[derive(Resource, Default)]
pub struct Screens {
    ids: FxHashMap<TypeId, ScreenData>,
    current: Option<TypeId>,
    default_to: Option<Box<dyn FnOnce(&mut World) + Send + Sync + 'static>>,
}

impl Screens {
    /// Returns true if the current screen is of type S
    #[inline]
    pub fn is_current<S: Screen>(&self) -> bool {
        self.current == Some(TypeId::of::<S>())
    }

    /// Returns true if the screen of type S is registered
    #[inline]
    pub fn contains<S: Screen>(&self) -> bool {
        self.ids.contains_key(&TypeId::of::<S>())
    }

    /// Returns the name of the current screen, if any
    #[inline]
    pub fn current_name(&self) -> Option<&'static str> {
        self.current
            .and_then(|id| self.ids.get(&id).map(|data| data.name))
    }

    #[inline]
    pub(crate) fn add_screen<S: Screen>(&mut self, world: &mut World) -> bool {
        if self.ids.contains_key(&TypeId::of::<S>()) {
            return false;
        }

        init_schedule_if_necessary(world, OnEnter::<S>::default());
        init_schedule_if_necessary(world, OnExit::<S>::default());

        self.ids.insert(
            TypeId::of::<S>(),
            ScreenData {
                name: S::name(),
                destroy_on_exit: false,
                on_enter: Arc::new(move |world: &mut World| {
                    log::debug!("Screen: on enter {}", S::name());
                    world.run_schedule(OnEnter::<S>::default());
                }),
                on_enter_from: Arc::new(move |world: &mut World, from: TypeId| {
                    let to = TypeId::of::<S>();
                    let _ = world.try_run_schedule(OnEnterFrom { from, to });
                }),
                on_exit: Arc::new(move |world: &mut World| {
                    log::debug!("Screen: on exit {}", S::name());
                    world.run_schedule(OnExit::<S>::default());
                }),
                on_exit_to: Arc::new(move |world: &mut World, to: TypeId| {
                    let from = TypeId::of::<S>();
                    let _ = world.try_run_schedule(OnExitTo { from, to });
                }),
            },
        );

        true
    }

    #[inline]
    pub(crate) fn set_current<S: Screen>(&mut self) {
        log::debug!("Screen: set current {}", S::name());
        self.current = Some(TypeId::of::<S>());
    }

    #[inline]
    pub(crate) fn clear_current(&mut self) {
        self.current = None;
    }

    #[inline]
    pub(crate) fn set_default<S: Screen>(&mut self) {
        self.default_to = Some(Box::new(|world| {
            log::debug!("Screen: set default to {}", S::name());
            world.screen::<S>().set_as_current();
        }));
    }
}

pub struct AppScreen<'w, S: Screen> {
    pub world: &'w mut World,
    pub screen: PhantomData<S>,
}

impl<'w, S: Screen> AppScreen<'w, S> {
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
                schedule.configure_sets(S::sys_set().run_if(is_in_screen::<S>));
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
        self.on_schedule(OnEnter::<S>::default(), systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_enter_from<FS: Screen, M>(
        &mut self,
        _from: FS,
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
        self.on_schedule(OnExit::<S>::default(), systems)
    }

    #[inline]
    #[track_caller]
    pub fn on_exit_to<TS: Screen, M>(
        &mut self,
        _to: TS,
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

    // Despawn all entities belonging to this screen when the screen is exited.
    #[inline]
    pub fn destroy_on_exit(&mut self) -> &mut Self {
        self.world
            .resource_mut::<Screens>()
            .ids
            .get_mut(&TypeId::of::<S>())
            .unwrap()
            .destroy_on_exit = true;
        self
    }
}

#[inline(always)]
pub fn is_in_screen<S: Screen>(screens: Option<Res<Screens>>) -> bool {
    screens.is_some_and(|screens| screens.is_current::<S>())
}

#[inline(always)]
pub fn no_screen_set(screens: Option<Res<Screens>>) -> bool {
    screens.is_none_or(|screens| screens.current.is_none())
}

pub(crate) fn change_screen_event_system<S: Screen>(world: &mut World) {
    world.resource_scope(|world, evt: Mut<Messages<ChangeScreenMsg<S>>>| {
        if evt.is_empty() {
            return;
        }

        let mut cursor = evt.get_cursor();
        for _ in cursor.read(&evt) {
            world.screen::<S>().set_as_current();
        }
    });
}

pub(crate) fn set_default_screen_event_system(world: &mut World) {
    if let Some(default_to) = world
        .get_resource_mut::<Screens>()
        .and_then(|mut screens| screens.default_to.take())
    {
        default_to(world);
    }
}

pub(crate) fn clear_screen_event_system(world: &mut World) {
    world.resource_scope(|world, evt: Mut<Messages<ClearScreenMsg>>| {
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
pub(crate) struct ClearScreenMsg;

#[derive(Clone, Copy, Default)]
pub(crate) struct ChangeScreen<S: Screen>(pub PhantomData<S>);

impl<S: Screen> Command for ChangeScreen<S> {
    fn apply(self, world: &mut World) {
        world.write_message(ChangeScreenMsg::<S>(PhantomData));
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct ClearScreen;

impl Command for ClearScreen {
    fn apply(self, world: &mut World) {
        world.write_message(ClearScreenMsg);
    }
}

pub trait ScreenCmdExt<'c, 'w, 's> {
    fn set_screen<S: Screen>(&'c mut self);
    fn clear_screen(&'c mut self);
    fn screen<S: Screen>(&'c mut self) -> ScreenCommands<'c, 'w, 's, S>;
}

impl<'c, 'w, 's> ScreenCmdExt<'c, 'w, 's> for Commands<'w, 's> {
    fn set_screen<S: Screen>(&'c mut self) {
        self.queue(ChangeScreen(PhantomData::<S>));
    }

    fn clear_screen(&'c mut self) {
        self.queue(ClearScreen);
    }

    fn screen<S: Screen>(&'c mut self) -> ScreenCommands<'c, 'w, 's, S> {
        ScreenCommands {
            cmds: self,
            _screen: PhantomData,
        }
    }
}

pub struct ScreenWorld<'w, S: Screen> {
    world: &'w mut World,
    _screen: PhantomData<S>,
}

impl<'w, S: Screen> ScreenWorld<'w, S> {
    /// Spawns a new entity with the given bundle and marks it as belonging to this screen.
    #[inline]
    #[track_caller]
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityWorldMut<'_> {
        self.world.spawn((InScreen::<S>::default(), bundle))
    }

    /// Spawns a new empty entity and marks it as belonging to this screen.
    #[inline]
    #[track_caller]
    pub fn spawn_empty(&mut self) -> EntityWorldMut<'_> {
        let mut e = self.world.spawn_empty();
        e.insert(InScreen::<S>::default());
        e
    }

    /// Spawns a batch of entities, each marked as belonging to this screen.
    #[inline]
    #[track_caller]
    pub fn spawn_batch<I>(&mut self, iter: I) -> impl Iterator<Item = Entity>
    where
        I: IntoIterator,
        I::Item: Bundle<Effect: NoBundleEffect>,
    {
        let batch = iter.into_iter().map(|b| (InScreen::<S>::default(), b));
        self.world.spawn_batch(batch)
    }

    /// Returns true if the given entity belongs to this screen.
    #[inline]
    #[track_caller]
    pub fn contains_entity(&mut self, entity: Entity) -> bool {
        self.get_entity(entity).is_some()
    }

    /// Attaches this screen's marker component to the given entity if it is not already attached.
    #[inline]
    #[track_caller]
    pub fn attach(&mut self, entity: Entity) {
        let Some(mut e) = self.get_entity_mut(entity) else {
            return;
        };
        e.insert_if_new(InScreen::<S>::default());
    }

    /// Detaches this screen's marker component from the given entity.
    #[inline]
    #[track_caller]
    pub fn detach(&mut self, entity: Entity) {
        let Some(mut e) = self.get_entity_mut(entity) else {
            return;
        };
        e.remove::<InScreen<S>>();
    }

    /// Despawns all entities belonging to this screen.
    #[inline]
    #[track_caller]
    pub fn despawn_all(&mut self) {
        // TODO: I am not sure if we can avoid the allocation here
        let entities = self
            .world
            .query_filtered::<Entity, With<InScreen<S>>>()
            .iter(self.world)
            .collect::<Vec<_>>();

        for e in entities {
            let _ = self.world.try_despawn(e);
        }
    }

    #[inline]
    #[track_caller]
    fn get_entity(&mut self, entity: Entity) -> Option<EntityRef<'_>> {
        let e = self.world.get_entity(entity).ok()?;
        e.contains::<InScreen<S>>().then_some(e)
    }

    #[inline]
    #[track_caller]
    fn get_entity_mut(&mut self, entity: Entity) -> Option<EntityWorldMut<'_>> {
        let e = self.world.get_entity_mut(entity).ok()?;
        e.contains::<InScreen<S>>().then_some(e)
    }

    fn set_as_current(&mut self) {
        let to_id = TypeId::of::<S>();
        let (from_id, from_actions, to_actions) = {
            let screens = self
                .world
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

        if let Some(fa) = &from_actions {
            (fa.on_exit_to)(self.world, to_id);
            (fa.on_exit)(self.world);
            if fa.destroy_on_exit {
                self.despawn_all();
            }
        }

        self.world.resource_mut::<Screens>().set_current::<S>();

        (to_actions.on_enter)(self.world);
        if let Some(fid) = from_id {
            (to_actions.on_enter_from)(self.world, fid);
        }
    }
}

pub trait ScreenWorldExt {
    fn screen<S: Screen>(&mut self) -> ScreenWorld<'_, S>;
}

impl ScreenWorldExt for World {
    fn screen<S: Screen>(&mut self) -> ScreenWorld<'_, S> {
        ScreenWorld {
            world: self,
            _screen: PhantomData,
        }
    }
}

pub struct ScreenCommands<'c, 'w, 's, S: Screen> {
    cmds: &'c mut Commands<'w, 's>,
    _screen: PhantomData<S>,
}

impl<'c, 'w, 's, S: Screen> ScreenCommands<'c, 'w, 's, S> {
    /// Spawn a bundle belonging to this screen
    #[inline]
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_> {
        self.cmds.spawn((InScreen::<S>::default(), bundle))
    }

    /// Spawn an empty entity belonging to this screen
    #[inline]
    pub fn spawn_empty(&mut self) -> EntityCommands<'_> {
        let mut ec = self.cmds.spawn_empty();
        ec.insert(InScreen::<S>::default());
        ec
    }

    /// Spawn a batch of bundles belonging to this screen
    /// Mirrors `Commands::spawn_batch` return type.
    #[inline]
    pub fn spawn_batch<I>(&mut self, iter: I)
    where
        I: IntoIterator + Send + Sync + 'static,
        I::Item: Bundle<Effect: NoBundleEffect>,
    {
        self.cmds.queue(move |world: &mut World| {
            let _ = world.screen::<S>().spawn_batch(iter);
        });
    }

    /// Ensure an entity is attached to this screen
    #[inline]
    pub fn attach(&mut self, entity: Entity) {
        self.cmds.entity(entity).insert(InScreen::<S>::default());
    }

    /// Detach an entity from this screen
    #[inline]
    pub fn detach(&mut self, entity: Entity) {
        self.cmds.entity(entity).remove::<InScreen<S>>();
    }

    /// Despawn all entities of this screen
    #[inline]
    pub fn despawn_all(&mut self) {
        self.cmds.queue(|world: &mut World| {
            world.screen::<S>().despawn_all();
        });
    }
}
