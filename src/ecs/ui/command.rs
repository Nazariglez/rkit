use crate::math::Vec2;
use bevy_ecs::prelude::*;
use rustc_hash::FxHashMap;
use taffy::prelude::*;

use super::{
    components::UINode,
    layout::{UILayout, UILayoutId},
    style::UIStyle,
};

pub struct SpawnUICommandBuilder<'c, 'w, 's, T>
where
    T: Send + Sync,
{
    cmds: Option<&'c mut Commands<'w, 's>>,
    current_entity: Entity,
    stack: Vec<Entity>,
    layout: UILayoutId<T>,
    bundles: Option<Vec<Box<dyn FnOnce(&mut World, &mut FxHashMap<Entity, NodeId>) + Send>>>,
}

pub struct SpawnUICommand {
    bundles: Vec<Box<dyn FnOnce(&mut World, &mut FxHashMap<Entity, NodeId>) + Send>>,
}

impl<T> SpawnUICommandBuilder<'_, '_, '_, T>
where
    T: Send + Sync + 'static,
{
    pub fn add<B: Bundle>(&mut self, bundle: B) -> &mut Self {
        let entity = self.cmds.as_mut().unwrap().spawn_empty().id();
        self.current_entity = entity;
        let parent_id = self.stack.last().cloned();
        self.bundles.as_mut().unwrap().push(Box::new(
            move |world: &mut World, ids: &mut FxHashMap<Entity, NodeId>| {
                let style = world
                    .entity_mut(entity)
                    .insert(bundle)
                    .insert_if_new(UIStyle::default())
                    .get_components::<(Entity, &UIStyle)>()
                    .map(|(e, style)| style.to_taffy())
                    .unwrap();

                let mut layout = world.get_resource_mut::<UILayout<T>>().unwrap();
                let parent_id = parent_id.and_then(|p_id| ids.get(&p_id)).cloned();
                let node_id = layout.add_node(entity, style, parent_id);
                ids.insert(entity, node_id);

                world.entity_mut(entity).insert(UINode {
                    layout: UILayoutId::<T>::default(),
                    node_id,
                    position: Vec2::ZERO,
                    size: Vec2::ONE,
                });
            },
        ));
        self
    }

    pub fn with_children<F: FnOnce(&mut Self)>(&mut self, cb: F) -> &mut Self {
        self.stack.push(self.current_entity);
        cb(self);
        self.stack.pop();
        self
    }
}

impl<T> Drop for SpawnUICommandBuilder<'_, '_, '_, T>
where
    T: Send + Sync,
{
    fn drop(&mut self) {
        let bundles = self.bundles.take();
        let command = SpawnUICommand {
            bundles: bundles.unwrap(),
        };
        let cmds = self.cmds.take().unwrap();
        cmds.queue(command);
    }
}

pub trait CommandSpawnUIExt<'w, 's> {
    fn spawn_ui<'c, T, B>(
        &'c mut self,
        layout: T,
        bundle: B,
    ) -> SpawnUICommandBuilder<'c, 'w, 's, T>
    where
        T: Send + Sync + 'static,
        B: Bundle;
}

impl<'w, 's> CommandSpawnUIExt<'w, 's> for Commands<'w, 's> {
    fn spawn_ui<'c, T, B>(
        &'c mut self,
        layout: T,
        bundle: B,
    ) -> SpawnUICommandBuilder<'c, 'w, 's, T>
    where
        T: Send + Sync + 'static,
        B: Bundle,
    {
        let mut builder = SpawnUICommandBuilder {
            cmds: Some(self),
            current_entity: Entity::from_raw(0),
            stack: vec![],
            bundles: Some(vec![]),
            layout: UILayoutId::<T>::default(),
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
