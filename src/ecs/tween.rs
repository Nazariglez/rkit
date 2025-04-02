use bevy_ecs::prelude::*;

use crate::tween::*;

use super::{app::App, prelude::OnUpdate, time::Time};

#[derive(SystemSet, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TweenSysSet;

pub struct TweenPlugin<C, T>
where
    C: Component,
    T: TweenableComponent<C>,
{
    _m: std::marker::PhantomData<(C, T)>,
}

impl<C, T> Default for TweenPlugin<C, T>
where
    C: Component,
    T: TweenableComponent<C>,
{
    fn default() -> Self {
        Self {
            _m: std::marker::PhantomData::<(C, T)>,
        }
    }
}

pub trait TweenableComponent<C: Component>: Send + Sync + 'static {
    fn tick(&mut self, target: &mut C, progress: f32);
}

#[derive(Event)]
pub struct TweenDone<T> {
    _m: std::marker::PhantomData<T>,
    pub id: Option<String>,
    pub entity: Entity,
}

#[derive(Component)]
pub struct ComponentTween<C: Component, T: TweenableComponent<C>> {
    _m: std::marker::PhantomData<C>,

    id: Option<String>,
    tween: Tween<f32>,
    tweenable: T,
}

impl<C, T> ComponentTween<C, T>
where
    C: Component,
    T: TweenableComponent<C>,
{
    pub fn from_raw(id: Option<String>, tween: Tween<f32>, tweenable: T) -> Self {
        Self {
            _m: std::marker::PhantomData,
            id,
            tween,
            tweenable,
        }
    }
}

fn add_component_tween<C: Component, T: TweenableComponent<C>>(app: &mut App) {
    app.add_event::<TweenDone<T>>()
        .add_systems(OnUpdate, tween_system::<C, T>.in_set(TweenSysSet));
}

fn tween_system<C: Component, T: TweenableComponent<C>>(
    mut cmds: Commands,
    mut query: Query<(Entity, &mut C, &mut ComponentTween<C, T>)>,
    time: Res<Time>,
    mut evt: EventWriter<TweenDone<T>>,
) {
    let dt = time.delta_f32();
    query
        .iter_mut()
        .for_each(|(entity, mut component, mut tween)| {
            tween.tween.tick(dt);

            let progress = tween.tween.value();
            tween.tweenable.tick(&mut component, progress);

            if tween.tween.is_ended() {
                cmds.entity(entity).remove::<ComponentTween<C, T>>();
                evt.send(TweenDone {
                    _m: std::marker::PhantomData::<T>,
                    id: tween.id.take(),
                    entity,
                });
            }
        });
}

pub trait TweenEntityCommandExt<'a> {
    fn tween<C: Component, T: TweenableComponent<C>>(
        &'a mut self,
        tweenbale: T,
        time: f32,
    ) -> TweenCmd<'a, C, T>;
}

pub struct TweenCmd<'a, C, T>
where
    C: Component,
    T: TweenableComponent<C>,
{
    _m: std::marker::PhantomData<C>,
    id: Option<String>,
    tween: Tween<f32>,
    e_cmds: &'a mut EntityCommands<'a>,
    tweenable: Option<T>,
}

impl<C, T> Drop for TweenCmd<'_, C, T>
where
    C: Component,
    T: TweenableComponent<C>,
{
    fn drop(&mut self) {
        self.e_cmds.insert(ComponentTween {
            _m: Default::default(),
            id: self.id.take(),
            tween: self.tween.start(),
            tweenable: self.tweenable.take().unwrap(),
        });
    }
}

impl<C, T> TweenCmd<'_, C, T>
where
    C: Component,
    T: TweenableComponent<C>,
{
    pub fn id(&mut self, id: &str) -> &mut Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn repeat(&mut self, repeat: u32) -> &mut Self {
        self.tween = self.tween.with_repeat(repeat);
        self
    }

    pub fn easing(&mut self, easing: EaseFn) -> &mut Self {
        self.tween = self.tween.with_easing(easing);
        self
    }

    pub fn yoyo(&mut self, yoyo: bool) -> &mut Self {
        self.tween = self.tween.with_yoyo(yoyo);
        self
    }

    pub fn infinite(&mut self, infinite: bool) -> &mut Self {
        self.tween = self.tween.with_loop(infinite);
        self
    }
}

impl<'a> TweenEntityCommandExt<'a> for EntityCommands<'a> {
    fn tween<C: Component, T: TweenableComponent<C>>(
        &'a mut self,
        tweenable: T,
        time: f32,
    ) -> TweenCmd<'a, C, T> {
        TweenCmd {
            tween: Tween::new(0.0f32, 1.0, time),
            id: None,
            e_cmds: self,
            _m: Default::default(),
            tweenable: Some(tweenable),
        }
    }
}
