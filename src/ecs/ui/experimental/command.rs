use bevy_ecs::prelude::*;

use super::{
    super::{
        command::SpawnUICommand,
        spawn::{UISpawnEntry, UISpawnParent, UISpawnPlan, UISpawnRelink, cleanup_owned_entities},
    },
    UIEntityScope, UIScene,
};

/// Deferred commands for materializing and moving owned ECS UI scenes.
pub trait CommandUISceneExt<'w, 's> {
    fn spawn_ui<T>(&mut self, layout: T, scene: UIScene) -> Entity
    where
        T: Component + Copy;

    /// Appends scene roots beneath an existing managed parent in source order.
    ///
    /// The parent must belong to `layout`; otherwise the whole batch is rejected without
    /// materializing any child. For managed node-to-node reparenting, insert Bevy's
    /// [`ChildOf`](bevy_ecs::hierarchy::ChildOf) directly after ensuring both nodes belong to the
    /// same layout. Use [`Self::make_ui_root`] to append a managed node at the private layout root.
    fn spawn_ui_children<T, I>(&mut self, layout: T, parent: Entity, children: I) -> Vec<Entity>
    where
        T: Component + Copy,
        I: IntoIterator<Item = UIScene>;

    /// Appends an existing managed node to the private root of `layout`.
    fn make_ui_root<T>(&mut self, layout: T, entity: Entity)
    where
        T: Component + Copy;
}

impl<'w, 's> CommandUISceneExt<'w, 's> for Commands<'w, 's> {
    fn spawn_ui<T>(&mut self, layout: T, scene: UIScene) -> Entity
    where
        T: Component + Copy,
    {
        let mut lowerer = SceneLowerer::new(self);
        let root = lowerer.lower_scene(scene, UISpawnParent::Root);
        lowerer.queue(layout);
        root
    }

    fn spawn_ui_children<T, I>(&mut self, layout: T, parent: Entity, children: I) -> Vec<Entity>
    where
        T: Component + Copy,
        I: IntoIterator<Item = UIScene>,
    {
        let mut lowerer = SceneLowerer::new(self);
        let mut roots = Vec::new();
        for child in children {
            roots.push(lowerer.lower_scene(child, UISpawnParent::Existing(parent)));
        }
        lowerer.queue(layout);
        roots
    }

    fn make_ui_root<T>(&mut self, layout: T, entity: Entity)
    where
        T: Component + Copy,
    {
        self.queue(SpawnUICommand::from_plan(UISpawnPlan::experimental(
            Vec::new(),
            vec![UISpawnRelink {
                entity,
                parent: UISpawnParent::Root,
            }],
            Vec::new(),
            layout,
        )));
    }
}

struct SceneLowerer<'c, 'w, 's> {
    commands: &'c mut Commands<'w, 's>,
    entries: Vec<UISpawnEntry>,
    allocations: Vec<Entity>,
}

impl<'c, 'w, 's> SceneLowerer<'c, 'w, 's> {
    fn new(commands: &'c mut Commands<'w, 's>) -> Self {
        Self {
            commands,
            entries: Vec::new(),
            allocations: Vec::new(),
        }
    }

    fn lower_scene(&mut self, scene: UIScene, parent: UISpawnParent) -> Entity {
        let UIScene {
            operations,
            observers,
            children,
            binding,
            ..
        } = self.expand_scene(scene);
        let entity = binding.entity.unwrap_or_else(|| self.allocate());
        self.entries.push(UISpawnEntry::experimental(
            entity,
            parent,
            operations,
            observers,
            binding.duplicate,
        ));
        for child in children {
            self.lower_scene(child, UISpawnParent::Plan(entity));
        }
        entity
    }

    fn expand_scene(&mut self, mut scene: UIScene) -> UIScene {
        while let Some(factory) = scene.factory.take() {
            let UIScene {
                operations,
                observers,
                children,
                binding,
                ..
            } = scene;
            let mut inner = {
                let mut reserve = || self.allocate();
                let mut scope = UIEntityScope::new(&mut reserve);
                factory(&mut scope)
            };
            inner.operations.extend(operations);
            inner.observers.extend(observers);
            inner.children.extend(children);
            inner.binding.merge(binding);
            scene = inner;
        }
        scene
    }

    fn queue<T: Component + Copy>(mut self, layout: T) {
        if self.entries.is_empty() {
            return;
        }
        self.commands
            .queue(SpawnUICommand::from_plan(UISpawnPlan::experimental(
                std::mem::take(&mut self.entries),
                Vec::new(),
                std::mem::take(&mut self.allocations),
                layout,
            )));
    }

    fn allocate(&mut self) -> Entity {
        let entity = self.commands.spawn_empty().id();
        self.allocations.push(entity);
        entity
    }
}

impl Drop for SceneLowerer<'_, '_, '_> {
    fn drop(&mut self) {
        let allocations = std::mem::take(&mut self.allocations);
        if allocations.is_empty() {
            return;
        }
        self.commands.queue(move |world: &mut World| {
            cleanup_owned_entities(world, &allocations);
        });
    }
}
