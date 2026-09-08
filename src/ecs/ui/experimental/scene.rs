use bevy_ecs::{
    bundle::{BundleFromComponents, NoBundleEffect},
    prelude::{Bundle, Entity, EntityEvent},
    system::IntoObserverSystem,
};

use super::super::{
    events::UIClick,
    spawn::{InsertExperimentalBundle, PatchStyle, UIObserverInstaller, UISpawnOperation},
    style::UIStyle,
};

/// An owned, one-shot description of one ECS UI node and its descendants.
#[derive(Default)]
pub struct UIScene {
    pub(super) factory: Option<UIEntityFactory>,
    pub(super) operations: Vec<Box<dyn UISpawnOperation>>,
    pub(super) observers: Vec<UIObserverInstaller>,
    pub(super) children: Vec<Self>,
    pub(super) binding: UIEntityBinding,
}

type UIEntityFactory = Box<dyn FnOnce(&mut UIEntityScope<'_>) -> UIScene + Send>;

#[derive(Default)]
pub(super) struct UIEntityBinding {
    pub(super) entity: Option<Entity>,
    pub(super) duplicate: bool,
}

impl UIEntityBinding {
    pub(super) fn merge(&mut self, outer: Self) {
        self.duplicate |= outer.duplicate || (self.entity.is_some() && outer.entity.is_some());
        if self.entity.is_none() {
            self.entity = outer.entity;
        }
    }
}

/// A short-lived capability for reserving entities while a UI scene is spawned.
///
/// It is available only while a [`super::ui::with_entities`] factory expands and cannot access
/// the ECS world or command buffer directly.
pub struct UIEntityScope<'a> {
    reserve: &'a mut dyn FnMut() -> Entity,
}

impl<'a> UIEntityScope<'a> {
    pub(crate) fn new(reserve: &'a mut dyn FnMut() -> Entity) -> Self {
        Self { reserve }
    }

    /// Reserves an entity for exactly one node in this spawn invocation.
    ///
    /// Every reservation must be assigned with [`UIScene::entity`] before the scene
    /// materializes, or the entire spawn invocation is rejected.
    pub fn reserve(&mut self) -> Entity {
        (self.reserve)()
    }
}

impl UIScene {
    pub fn node() -> Self {
        Self::default()
    }

    /// Assigns the eventual root node to an entity reserved by this spawn invocation.
    ///
    /// The entity is not adopted or otherwise modified until the scene successfully
    /// materializes. Assigning more than one entity to the effective root rejects the spawn.
    pub fn entity(mut self, entity: Entity) -> Self {
        self.binding.duplicate |= self.binding.entity.replace(entity).is_some();
        self
    }

    pub fn insert<B>(mut self, bundle: B) -> Self
    where
        B: Bundle + BundleFromComponents,
        B::Effect: NoBundleEffect,
    {
        self.operations
            .push(Box::new(InsertExperimentalBundle::new(bundle)));
        self
    }

    pub fn style<F>(mut self, patch: F) -> Self
    where
        F: FnOnce(UIStyle) -> UIStyle + Send + 'static,
    {
        self.operations.push(Box::new(PatchStyle::new(patch)));
        self
    }

    /// Installs a native entity-scoped observer after this scene materializes.
    ///
    /// Multiple local observers have no specified order. Prefer action components with a global
    /// observer for repeated controls so observer entities are created only where needed.
    pub fn observe<E, B, M>(mut self, observer: impl IntoObserverSystem<E, B, M>) -> Self
    where
        E: EntityEvent,
        B: Bundle,
    {
        self.observers.push(Box::new(move |world, entity| {
            world.entity_mut(entity).observe::<E, B, M>(observer);
        }));
        self
    }

    /// Installs a native exact-target [`UIClick`] observer after this scene materializes.
    pub fn on_click<B, M>(self, observer: impl IntoObserverSystem<UIClick, B, M>) -> Self
    where
        B: Bundle,
    {
        self.observe(observer)
    }

    pub fn child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }

    pub fn children<I>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        self.children.extend(children);
        self
    }
}
