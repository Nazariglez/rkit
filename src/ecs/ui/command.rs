use crate::macros::Deref;
use bevy_ecs::prelude::*;

use super::spawn::{
    UISpawnEntry, UISpawnPlan, clear_ui_children, despawn_ui_node, reparent_ui_node,
};

pub struct SpawnUICommand<T: Component> {
    plan: UISpawnPlan<T>,
}

impl<T: Component> SpawnUICommand<T> {
    pub(crate) fn from_plan(plan: UISpawnPlan<T>) -> Self {
        Self { plan }
    }
}

pub struct AddUIChildCommand<T: Component> {
    _m: std::marker::PhantomData<T>,
    parent: Entity,
    child: Entity,
}

pub struct ClearUIChildrenCommand<T: Component> {
    _m: std::marker::PhantomData<T>,
    parent: Entity,
}

pub struct DespawnUICommand<T: Component> {
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
    entries: Option<Vec<UISpawnEntry>>,
}

impl<'c, 'w, 's, T: Component + Copy> SpawnUICommandBuilder<'c, 'w, 's, T> {
    pub fn add<B: Bundle>(&mut self, bundle: B) -> &mut Self {
        self.current_entity = self.cmds.spawn_empty().id();
        let entity = self.current_entity;
        self.entries
            .as_mut()
            .unwrap()
            .push(UISpawnEntry::compatibility(
                entity,
                self.stack.last().copied(),
                bundle,
            ));
        self
    }

    pub fn with_children<F: FnOnce(&mut Self)>(&mut self, cb: F) -> &mut Self {
        let previous = self.current_entity;
        self.stack.push(self.current_entity);
        cb(self);
        self.stack.pop();
        self.current_entity = previous;
        self
    }

    pub fn entity_id(&self) -> Entity {
        self.current_entity
    }
}

impl<T: Component + Copy> Drop for SpawnUICommandBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        self.cmds
            .queue(SpawnUICommand::from_plan(UISpawnPlan::compatibility(
                self.entries.take().unwrap(),
                self.layout,
            )));
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

    fn clear_ui_children<T>(&mut self, layout: T, parent: Entity)
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
            current_entity: Entity::from_raw_u32(0).unwrap(),
            stack: vec![],
            entries: Some(vec![]),
            layout,
        };
        builder.add(bundle);
        builder
    }

    fn add_ui_child<T: Component>(&mut self, _layout: T, parent: Entity, child: Entity) {
        self.queue(AddUIChildCommand {
            _m: std::marker::PhantomData::<T>,
            parent,
            child,
        });
    }

    fn clear_ui_children<T: Component>(&mut self, _layout: T, parent: Entity) {
        self.queue(ClearUIChildrenCommand {
            _m: std::marker::PhantomData::<T>,
            parent,
        });
    }

    fn despawn_ui_node<T: Component>(&mut self, _layout: T, entity: Entity) {
        self.queue(DespawnUICommand {
            _m: std::marker::PhantomData::<T>,
            entity,
        });
    }
}

impl<T: Component + Copy> Command for SpawnUICommand<T> {
    fn apply(self, world: &mut World) {
        self.plan.materialize(world);
    }
}

impl<T: Component> Command for AddUIChildCommand<T> {
    fn apply(self, world: &mut World) {
        reparent_ui_node::<T>(world, self.parent, self.child);
    }
}

impl<T: Component> Command for ClearUIChildrenCommand<T> {
    fn apply(self, world: &mut World) {
        clear_ui_children::<T>(world, self.parent);
    }
}

impl<T: Component> Command for DespawnUICommand<T> {
    fn apply(self, world: &mut World) {
        despawn_ui_node::<T>(world, self.entity);
    }
}
