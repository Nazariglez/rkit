use bevy_ecs::prelude::*;

use super::{
    super::{
        command::SpawnUICommand,
        spawn::{UISpawnEntry, UISpawnParent, UISpawnPlan, UISpawnRelink},
    },
    UIScene,
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
        let mut entries = Vec::new();
        let root = lower_scene(self, scene, UISpawnParent::Root, &mut entries);
        self.queue(SpawnUICommand::from_plan(UISpawnPlan::experimental(
            entries,
            Vec::new(),
            layout,
        )));
        root
    }

    fn spawn_ui_children<T, I>(&mut self, layout: T, parent: Entity, children: I) -> Vec<Entity>
    where
        T: Component + Copy,
        I: IntoIterator<Item = UIScene>,
    {
        let mut entries = Vec::new();
        let roots = children
            .into_iter()
            .map(|child| lower_scene(self, child, UISpawnParent::Existing(parent), &mut entries))
            .collect();
        if !entries.is_empty() {
            self.queue(SpawnUICommand::from_plan(UISpawnPlan::experimental(
                entries,
                Vec::new(),
                layout,
            )));
        }
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
            layout,
        )));
    }
}

fn lower_scene(
    commands: &mut Commands,
    scene: UIScene,
    parent: UISpawnParent,
    entries: &mut Vec<UISpawnEntry>,
) -> Entity {
    let entity = commands.spawn_empty().id();
    let UIScene {
        operations,
        observers,
        children,
    } = scene;
    entries.push(UISpawnEntry::experimental(
        entity, parent, operations, observers,
    ));
    for child in children {
        lower_scene(commands, child, UISpawnParent::Plan(entity), entries);
    }
    entity
}
