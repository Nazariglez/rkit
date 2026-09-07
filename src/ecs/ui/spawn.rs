use bevy_ecs::prelude::*;
use bevy_ecs::{
    bundle::{BundleFromComponents, NoBundleEffect},
    component::ComponentId,
};

use super::{
    components::UINode,
    ctx::UINodeType,
    layout::UILayoutOwner,
    plugin::{layout_root, refresh_layout},
    style::UIStyle,
};
use super::{
    diagnostics::{UIRuntimeError, UISceneError},
    layout::{UILayout, UILayoutRoot, valid_managed_branch, valid_managed_node},
    plugin::experimental_layout_root,
};

pub(crate) trait UISpawnOperation: Send {
    fn apply(self: Box<Self>, world: &mut World, entity: Entity);

    fn preflight(
        &self,
        _world: &mut World,
        _forbidden: &[ComponentId],
    ) -> Result<(), UISceneError> {
        Ok(())
    }
}

pub(crate) type UIObserverInstaller = Box<dyn FnOnce(&mut World, Entity) + Send>;

struct InsertCompatibilityBundle<B>(B);

impl<B: Bundle> UISpawnOperation for InsertCompatibilityBundle<B> {
    fn apply(self: Box<Self>, world: &mut World, entity: Entity) {
        world.entity_mut(entity).insert(self.0);
    }
}

pub(crate) struct InsertExperimentalBundle<B>(B);

impl<B> InsertExperimentalBundle<B> {
    pub(crate) fn new(bundle: B) -> Self {
        Self(bundle)
    }
}

impl<B> UISpawnOperation for InsertExperimentalBundle<B>
where
    B: Bundle + BundleFromComponents,
    B::Effect: NoBundleEffect,
{
    fn apply(self: Box<Self>, world: &mut World, entity: Entity) {
        world.entity_mut(entity).insert(self.0);
    }

    fn preflight(&self, world: &mut World, forbidden: &[ComponentId]) -> Result<(), UISceneError> {
        let components = world.register_bundle::<B>().contributed_components();
        if components
            .iter()
            .any(|component| forbidden.contains(component))
        {
            return Err(UISceneError::RuntimeOwnedComponent);
        }
        Ok(())
    }
}

pub(crate) struct PatchStyle<F>(F);

impl<F> PatchStyle<F> {
    pub(crate) fn new(patch: F) -> Self {
        Self(patch)
    }
}

impl<F> UISpawnOperation for PatchStyle<F>
where
    F: FnOnce(UIStyle) -> UIStyle + Send + 'static,
{
    fn apply(self: Box<Self>, world: &mut World, entity: Entity) {
        let style = *world.entity(entity).get::<UIStyle>().unwrap();
        world.entity_mut(entity).insert((self.0)(style));
    }
}

#[derive(Clone, Copy)]
pub(crate) enum UISpawnParent {
    Root,
    Plan(Entity),
    Existing(Entity),
}

impl UISpawnParent {
    fn resolve(self, root: Entity) -> Entity {
        match self {
            Self::Root => root,
            Self::Plan(entity) => entity,
            Self::Existing(entity) => entity,
        }
    }
}

pub(crate) struct UISpawnEntry {
    pub(crate) entity: Entity,
    pub(crate) parent: UISpawnParent,
    pub(crate) operations: Vec<Box<dyn UISpawnOperation>>,
    pub(crate) observers: Vec<UIObserverInstaller>,
}

impl UISpawnEntry {
    pub(crate) fn compatibility<B: Bundle>(
        entity: Entity,
        parent: Option<Entity>,
        bundle: B,
    ) -> Self {
        Self {
            entity,
            parent: parent.map_or(UISpawnParent::Root, UISpawnParent::Plan),
            operations: vec![Box::new(InsertCompatibilityBundle(bundle))],
            observers: Vec::new(),
        }
    }

    pub(crate) fn experimental(
        entity: Entity,
        parent: UISpawnParent,
        operations: Vec<Box<dyn UISpawnOperation>>,
        observers: Vec<UIObserverInstaller>,
    ) -> Self {
        Self {
            entity,
            parent,
            operations,
            observers,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct UISpawnRelink {
    pub(crate) entity: Entity,
    pub(crate) parent: UISpawnParent,
}

#[derive(Clone, Copy)]
enum UISpawnOrigin {
    Compatibility,
    Experimental,
}

pub(crate) struct UISpawnPlan<T: Component> {
    entries: Vec<UISpawnEntry>,
    relinks: Vec<UISpawnRelink>,
    layout: T,
    origin: UISpawnOrigin,
}

impl<T: Component + Copy> UISpawnPlan<T> {
    pub(crate) fn compatibility(entries: Vec<UISpawnEntry>, layout: T) -> Self {
        Self {
            entries,
            relinks: Vec::new(),
            layout,
            origin: UISpawnOrigin::Compatibility,
        }
    }

    pub(crate) fn experimental(
        entries: Vec<UISpawnEntry>,
        relinks: Vec<UISpawnRelink>,
        layout: T,
    ) -> Self {
        Self {
            entries,
            relinks,
            layout,
            origin: UISpawnOrigin::Experimental,
        }
    }

    pub(crate) fn materialize(mut self, world: &mut World) {
        let root = match self.origin {
            UISpawnOrigin::Compatibility => layout_root::<T>(world),
            UISpawnOrigin::Experimental => experimental_layout_root::<T>(world),
        };
        let Some(root) = root else {
            cleanup_scene_entries(world, &self.entries);
            if matches!(self.origin, UISpawnOrigin::Experimental) {
                report_missing_layout::<T>(world);
            }
            return;
        };

        if matches!(self.origin, UISpawnOrigin::Experimental) {
            let invalid_entity = self
                .entries
                .first()
                .map(|entry| entry.entity)
                .or_else(|| self.relinks.first().map(|relink| relink.entity));
            let forbidden = runtime_owned_components::<T>(world);
            if let Err(reason) = preflight_experimental_plan::<T>(
                world,
                &self.entries,
                &self.relinks,
                root,
                &forbidden,
            ) {
                cleanup_scene_entries(world, &self.entries);
                if let Some(entity) = invalid_entity {
                    report_invalid_scene(world, entity, reason);
                }
                return;
            }

            for entry in &self.entries {
                install_runtime_components(
                    world,
                    entry.entity,
                    self.layout,
                    root,
                    UISpawnOrigin::Experimental,
                );
            }
        }

        for entry in &mut self.entries {
            for operation in std::mem::take(&mut entry.operations) {
                operation.apply(world, entry.entity);
            }
            match self.origin {
                UISpawnOrigin::Compatibility => {
                    install_runtime_components(
                        world,
                        entry.entity,
                        self.layout,
                        root,
                        UISpawnOrigin::Compatibility,
                    );
                    link_entity(world, entry.entity, entry.parent, root);
                }
                UISpawnOrigin::Experimental => {
                    world
                        .entity_mut(entry.entity)
                        .insert_if_new(UINodeType::Container);
                }
            }
        }

        if matches!(self.origin, UISpawnOrigin::Experimental) {
            for entry in &self.entries {
                link_entity(world, entry.entity, entry.parent, root);
            }
        }
        for relink in self.relinks {
            link_entity(world, relink.entity, relink.parent, root);
        }
        for entry in self.entries {
            for observer in entry.observers {
                observer(world, entry.entity);
            }
        }

        match self.origin {
            UISpawnOrigin::Compatibility => refresh_layout::<T>(world),
            UISpawnOrigin::Experimental => {
                world.resource_mut::<UILayout<T>>().mark_topology_dirty();
            }
        }
    }
}

fn install_runtime_components<T: Component>(
    world: &mut World,
    entity: Entity,
    layout: T,
    root: Entity,
    origin: UISpawnOrigin,
) {
    let mut entity = world.entity_mut(entity);
    entity.insert((layout, UILayoutOwner { root }, UINode::new()));
    match origin {
        UISpawnOrigin::Compatibility => {
            entity
                .insert_if_new(UIStyle::default())
                .insert_if_new(UINodeType::Container);
        }
        UISpawnOrigin::Experimental => {
            entity.insert(UIStyle::default());
        }
    }
}

fn link_entity(world: &mut World, entity: Entity, parent: UISpawnParent, root: Entity) {
    world
        .entity_mut(entity)
        .insert(ChildOf(parent.resolve(root)));
}

fn preflight_experimental_plan<T: Component>(
    world: &mut World,
    entries: &[UISpawnEntry],
    relinks: &[UISpawnRelink],
    root: Entity,
    forbidden: &[ComponentId],
) -> Result<(), UISceneError> {
    let entries_by_entity = entries
        .iter()
        .map(|entry| entry.entity)
        .collect::<rustc_hash::FxHashSet<_>>();
    let mut validated_existing_parents = rustc_hash::FxHashSet::default();
    for entry in entries {
        if !world.entities().contains(entry.entity) {
            return Err(UISceneError::MissingEntity);
        }
        preflight_parent::<T>(
            world,
            entry.parent,
            &entries_by_entity,
            &mut validated_existing_parents,
            root,
        )?;
        for operation in &entry.operations {
            operation.preflight(world, forbidden)?;
        }
    }
    for relink in relinks {
        if !world.entities().contains(relink.entity) {
            return Err(UISceneError::MissingEntity);
        }
        if !valid_managed_node::<T>(world, relink.entity, root) {
            return Err(UISceneError::WrongLayout);
        }
        preflight_parent::<T>(
            world,
            relink.parent,
            &entries_by_entity,
            &mut validated_existing_parents,
            root,
        )?;
    }
    Ok(())
}

fn preflight_parent<T: Component>(
    world: &World,
    parent: UISpawnParent,
    entries: &rustc_hash::FxHashSet<Entity>,
    validated_existing: &mut rustc_hash::FxHashSet<Entity>,
    root: Entity,
) -> Result<(), UISceneError> {
    match parent {
        UISpawnParent::Root => Ok(()),
        UISpawnParent::Plan(parent) if entries.contains(&parent) => Ok(()),
        UISpawnParent::Plan(_) => Err(UISceneError::MissingParent),
        UISpawnParent::Existing(parent) if !world.entities().contains(parent) => {
            Err(UISceneError::MissingParent)
        }
        UISpawnParent::Existing(parent) if validated_existing.contains(&parent) => Ok(()),
        UISpawnParent::Existing(parent) if valid_managed_branch::<T>(world, parent, root) => {
            validated_existing.insert(parent);
            Ok(())
        }
        UISpawnParent::Existing(_) => Err(UISceneError::WrongLayout),
    }
}

fn runtime_owned_components<T: Component>(world: &mut World) -> [ComponentId; 6] {
    [
        world.register_component::<ChildOf>(),
        world.register_component::<Children>(),
        world.register_component::<UINode>(),
        world.register_component::<T>(),
        world.register_component::<UILayoutOwner>(),
        world.register_component::<UILayoutRoot<T>>(),
    ]
}

fn cleanup_scene_entries(world: &mut World, entries: &[UISpawnEntry]) {
    for entry in entries {
        let _ = world.try_despawn(entry.entity);
    }
}

fn report_missing_layout<T: Component>(world: &mut World) {
    let layout = std::any::type_name::<T>();
    log::error!("Cannot materialize ECS UI scene: layout {layout} is unavailable");
    world.trigger(UIRuntimeError::MissingLayout { layout });
}

fn report_invalid_scene(world: &mut World, root: Entity, reason: UISceneError) {
    log::error!("Cannot materialize ECS UI scene rooted at {root:?}: {reason:?}");
    world.trigger(UIRuntimeError::InvalidScene { root, reason });
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
