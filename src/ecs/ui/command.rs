use crate::math::Vec2;
use bevy_ecs::prelude::*;
use macros::Deref;
use rustc_hash::FxHashMap;
use taffy::prelude::*;

use super::{components::UINode, layout::UILayout, style::UIStyle};

type BuilderCb = dyn FnOnce(&mut World, &mut FxHashMap<Entity, NodeId>) + Send;

pub struct SpawnUICommandBuilder<'c, 'w, 's, T>
where
    T: Component + Copy,
{
    cmds: &'c mut Commands<'w, 's>,
    current_entity: Entity,
    stack: Vec<Entity>,
    layout: T,
    bundles: Option<Vec<Box<BuilderCb>>>,
}

pub struct SpawnUICommand {
    bundles: Vec<Box<BuilderCb>>,
}

#[derive(Deref)]
pub struct SpawnUICommandEntity<'temp, 'c, 'w, 's, T>
where
    T: Component + Copy,
{
    #[deref]
    cmd: &'temp mut SpawnUICommandBuilder<'c, 'w, 's, T>,
    entity: Entity,
}

impl<T> SpawnUICommandEntity<'_, '_, '_, '_, T>
where
    T: Component + Copy,
{
    pub fn entity_id(self) -> Entity {
        self.entity
    }
}

impl<'c, 'w, 's, T> SpawnUICommandBuilder<'c, 'w, 's, T>
where
    T: Component + Copy,
{
    pub fn add<'temp, B: Bundle>(
        &'temp mut self,
        bundle: B,
    ) -> SpawnUICommandEntity<'temp, 'c, 'w, 's, T> {
        self.current_entity = self.cmds.spawn_empty().id();
        let entity = self.current_entity;
        let parent_id = self.stack.last().cloned();
        let layout = self.layout;
        self.bundles.as_mut().unwrap().push(Box::new(
            move |world: &mut World, ids: &mut FxHashMap<Entity, NodeId>| {
                let style = world
                    .entity_mut(entity)
                    .insert((layout, bundle))
                    .insert_if_new(UIStyle::default())
                    .get_components::<&UIStyle>()
                    .map(|style| style.to_taffy())
                    .unwrap();

                let mut layout = world.get_resource_mut::<UILayout<T>>().unwrap();
                let parent_id = parent_id.and_then(|p_id| ids.get(&p_id)).cloned();
                let node_id = layout.add_node(entity, style, parent_id);
                ids.insert(entity, node_id);

                world.entity_mut(entity).insert(UINode {
                    node_id,
                    position: Vec2::ZERO,
                    size: Vec2::ONE,
                });
            },
        ));

        SpawnUICommandEntity { cmd: self, entity }
    }

    pub fn with_children<'temp, F: FnOnce(&mut Self)>(
        &'temp mut self,
        cb: F,
    ) -> SpawnUICommandEntity<'temp, 'c, 'w, 's, T> {
        let entity = self.current_entity;

        self.stack.push(self.current_entity);
        cb(self);
        self.stack.pop();

        SpawnUICommandEntity { cmd: self, entity }
    }

    pub fn entity_id(self) -> Entity {
        self.stack.last().cloned().unwrap_or(self.current_entity)
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
