use crate::macros::Deref;
use crate::math::{Mat3, Vec2};
use crate::prelude::PanicContext;
use bevy_ecs::prelude::*;
use rustc_hash::FxHashMap;
use taffy::prelude::*;

use super::ctx::UINodeType;
use super::{components::UINode, layout::UILayout, style::UIStyle};

type BuilderCb = dyn FnOnce(&mut World, &mut FxHashMap<Entity, NodeId>) + Send;

pub struct SpawnUICommand {
    bundles: Vec<Box<BuilderCb>>,
}

pub struct AddUIChildCommand<T>
where
    T: Component,
{
    _m: std::marker::PhantomData<T>,
    parent: Entity,
    child: Entity,
}

pub struct DespawnUICommand<T>
where
    T: Component,
{
    _m: std::marker::PhantomData<T>,
    entity: Entity,
}

#[derive(Deref)]
pub struct SpawnUICommandBuilder<'c, 'w, 's, T>
where
    T: Component + Copy,
{
    #[deref]
    pub cmds: &'c mut Commands<'w, 's>,
    current_entity: Entity,
    stack: Vec<Entity>,
    layout: T,
    bundles: Option<Vec<Box<BuilderCb>>>,
}

impl<'c, 'w, 's, T> SpawnUICommandBuilder<'c, 'w, 's, T>
where
    T: Component + Copy,
{
    pub fn add<B: Bundle>(&mut self, bundle: B) -> &mut SpawnUICommandBuilder<'c, 'w, 's, T> {
        self.current_entity = self.cmds.spawn_empty().id();
        let entity = self.current_entity;
        let parent_id = self.stack.last().cloned();
        let layout = self.layout;
        self.bundles.as_mut().unwrap().push(Box::new(
            move |world: &mut World, ids: &mut FxHashMap<Entity, NodeId>| {
                let (style, typ) = world
                    .entity_mut(entity)
                    .insert((layout, bundle))
                    .insert_if_new(UIStyle::default())
                    .insert_if_new(UINodeType::Container)
                    .get_components::<(&UIStyle, &UINodeType)>()
                    .map(|(style, typ)| (style.as_taffy_style(), *typ))
                    .unwrap();

                let mut layout = world
                    .get_resource_mut::<UILayout<T>>()
                    .or_panic("Cannot find UILayout to add Nodes. Are you sure the name of the layout is right or that it was initialized?");

                let parent_id = parent_id.and_then(|p_id| ids.get(&p_id)).cloned();
                let node_id = layout.add_raw_node(entity, style, typ, parent_id);
                ids.insert(entity, node_id);

                world.entity_mut(entity).insert(UINode {
                    node_id,
                    position: Vec2::ZERO,
                    size: Vec2::ONE,

                    local_transform: Mat3::IDENTITY,
                    global_transform: Mat3::IDENTITY,
                    parent_global_transform: Mat3::IDENTITY,

                    global_alpha: 1.0,
                });
            },
        ));

        self
    }

    pub fn with_children<F: FnOnce(&mut Self)>(
        &mut self,
        cb: F,
    ) -> &mut SpawnUICommandBuilder<'c, 'w, 's, T> {
        let prev = self.current_entity;
        self.stack.push(self.current_entity);
        cb(self);
        self.stack.pop();
        self.current_entity = prev;

        self
    }

    pub fn entity_id(&self) -> Entity {
        self.current_entity
    }
}

impl<T> Drop for SpawnUICommandBuilder<'_, '_, '_, T>
where
    T: Component + Copy,
{
    fn drop(&mut self) {
        let bundles = self.bundles.take();
        let command = SpawnUICommand {
            bundles: bundles.unwrap(),
        };
        self.cmds.queue(command);
    }
}

pub trait CommandSpawnUIExt<'w, 's> {
    fn spawn_ui_node<'c, T, B>(
        &'c mut self,
        layout: T,
        bundle: B,
    ) -> SpawnUICommandBuilder<'c, 'w, 's, T>
    where
        T: Component + Copy,
        B: Bundle;

    fn despawn_ui_node<T>(&mut self, layout: T, entity: Entity)
    where
        T: Component;

    fn add_ui_child<T>(&mut self, layout: T, parent: Entity, child: Entity)
    where
        T: Component;
}

impl<'w, 's> CommandSpawnUIExt<'w, 's> for Commands<'w, 's> {
    fn spawn_ui_node<'c, T, B>(
        &'c mut self,
        layout: T,
        bundle: B,
    ) -> SpawnUICommandBuilder<'c, 'w, 's, T>
    where
        T: Component + Copy,
        B: Bundle,
    {
        let mut builder = SpawnUICommandBuilder {
            cmds: self,
            current_entity: Entity::from_raw(0),
            stack: vec![],
            bundles: Some(vec![]),
            layout,
        };

        builder.add(bundle);
        builder
    }

    fn add_ui_child<T>(&mut self, _layout: T, parent: Entity, child: Entity)
    where
        T: Component,
    {
        self.queue(AddUIChildCommand {
            _m: std::marker::PhantomData::<T>,
            parent,
            child,
        });
    }

    fn despawn_ui_node<T>(&mut self, _layout: T, entity: Entity)
    where
        T: Component,
    {
        self.queue(DespawnUICommand {
            _m: std::marker::PhantomData::<T>,
            entity,
        });
    }
}

impl Command for SpawnUICommand {
    fn apply(self, world: &mut World) {
        let Self { bundles } = self;
        let mut table = FxHashMap::default();
        for cb in bundles {
            cb(world, &mut table);
        }
    }
}

impl<T> Command for AddUIChildCommand<T>
where
    T: Component,
{
    fn apply(self, world: &mut World) {
        let Self { _m, parent, child } = self;
        let mut layout = world.get_resource_mut::<UILayout<T>>().unwrap();
        layout.add_child(parent, child);
    }
}

impl<T> Command for DespawnUICommand<T>
where
    T: Component,
{
    fn apply(self, world: &mut World) {
        let Self {
            _m: _layout,
            entity,
        } = self;
        let layout = world.get_resource::<UILayout<T>>().unwrap();
        layout.tree_from_node(entity).iter().for_each(|e| {
            let _ = world.despawn(*e);
        });
    }
}
