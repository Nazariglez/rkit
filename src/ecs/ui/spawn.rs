use bevy_ecs::prelude::*;

use super::{
    components::UINode,
    ctx::UINodeType,
    layout::UILayoutOwner,
    plugin::{layout_root, refresh_layout},
    style::UIStyle,
};

pub(crate) trait UISpawnOperation: Send {
    fn apply(self: Box<Self>, world: &mut World, entity: Entity);
}

struct InsertCompatibilityBundle<B>(B);

impl<B: Bundle> UISpawnOperation for InsertCompatibilityBundle<B> {
    fn apply(self: Box<Self>, world: &mut World, entity: Entity) {
        world.entity_mut(entity).insert(self.0);
    }
}

pub(crate) struct UISpawnEntry {
    pub(crate) entity: Entity,
    pub(crate) parent: Option<Entity>,
    pub(crate) operations: Vec<Box<dyn UISpawnOperation>>,
}

impl UISpawnEntry {
    pub(crate) fn compatibility<B: Bundle>(
        entity: Entity,
        parent: Option<Entity>,
        bundle: B,
    ) -> Self {
        Self {
            entity,
            parent,
            operations: vec![Box::new(InsertCompatibilityBundle(bundle))],
        }
    }
}

pub(crate) struct UISpawnPlan<T: Component> {
    entries: Vec<UISpawnEntry>,
    layout: T,
}

impl<T: Component + Copy> UISpawnPlan<T> {
    pub(crate) fn compatibility(entries: Vec<UISpawnEntry>, layout: T) -> Self {
        Self { entries, layout }
    }

    pub(crate) fn materialize(self, world: &mut World) {
        let Self { entries, layout } = self;
        let Some(root) = layout_root::<T>(world) else {
            for entry in entries {
                let _ = world.try_despawn(entry.entity);
            }
            return;
        };

        for entry in entries {
            for operation in entry.operations {
                operation.apply(world, entry.entity);
            }
            let parent = entry.parent.unwrap_or(root);
            let mut entity = world.entity_mut(entry.entity);
            entity
                .insert_if_new(UIStyle::default())
                .insert_if_new(UINodeType::Container)
                .insert((
                    layout,
                    UILayoutOwner { root },
                    ChildOf(parent),
                    UINode::new(),
                ));
        }

        refresh_layout::<T>(world);
    }
}

pub(crate) fn reparent_ui_node<T: Component>(world: &mut World, parent: Entity, child: Entity) {
    let Some(root) = layout_root::<T>(world) else {
        return;
    };
    if !managed_by::<T>(world, parent, root) || !managed_by::<T>(world, child, root) {
        log::warn!("Ignoring UI child relation outside its layout: {parent:#?} -> {child:#?}");
        return;
    }
    if creates_cycle(world, parent, child) {
        log::warn!("Ignoring cyclic UI child relation: {parent:#?} -> {child:#?}");
        return;
    }
    world.entity_mut(child).insert(ChildOf(parent));
    refresh_layout::<T>(world);
}

pub(crate) fn clear_ui_children<T: Component>(world: &mut World, parent: Entity) {
    let Some(root) = layout_root::<T>(world) else {
        return;
    };
    if !managed_by::<T>(world, parent, root) {
        log::warn!("Ignoring UI clear outside its layout: {parent:#?}");
        return;
    }
    let children = world
        .get::<Children>(parent)
        .map(|children| children.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    let mut changed = false;
    for child in children {
        if !managed_by::<T>(world, child, root) {
            continue;
        }
        if !managed_subtree::<T>(world, child, root) {
            log::warn!(
                "Ignoring UI clear for a subtree with foreign or cyclic descendants: {child:#?}"
            );
            continue;
        }
        changed |= world.try_despawn(child).is_ok();
    }
    if changed {
        refresh_layout::<T>(world);
    }
}

pub(crate) fn despawn_ui_node<T: Component>(world: &mut World, entity: Entity) {
    let Some(root) = layout_root::<T>(world) else {
        return;
    };
    if !managed_by::<T>(world, entity, root) {
        log::warn!("Ignoring UI despawn outside its layout: {entity:#?}");
        return;
    }
    if !managed_subtree::<T>(world, entity, root) {
        log::warn!(
            "Ignoring UI despawn for a subtree with foreign or cyclic descendants: {entity:#?}"
        );
        return;
    }
    let _ = world.try_despawn(entity);
    refresh_layout::<T>(world);
}

fn managed_by<T: Component>(world: &World, entity: Entity, root: Entity) -> bool {
    world.get::<T>(entity).is_some()
        && world
            .get::<UILayoutOwner>(entity)
            .is_some_and(|owner| owner.root == root)
}

fn creates_cycle(world: &World, parent: Entity, child: Entity) -> bool {
    let mut ancestor = parent;
    let mut reached = rustc_hash::FxHashSet::default();
    loop {
        if ancestor == child || !reached.insert(ancestor) {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(ancestor) else {
            return false;
        };
        ancestor = parent.parent();
    }
}

fn managed_subtree<T: Component>(world: &World, entity: Entity, root: Entity) -> bool {
    let mut pending = vec![entity];
    let mut reached = rustc_hash::FxHashSet::default();
    while let Some(entity) = pending.pop() {
        if !reached.insert(entity) || !managed_by::<T>(world, entity, root) {
            return false;
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    true
}
