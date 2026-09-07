use bevy_ecs::{
    bundle::{BundleFromComponents, NoBundleEffect},
    prelude::{Bundle, EntityEvent},
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
    pub(super) operations: Vec<Box<dyn UISpawnOperation>>,
    pub(super) observers: Vec<UIObserverInstaller>,
    pub(super) children: Vec<Self>,
}

impl UIScene {
    pub fn node() -> Self {
        Self::default()
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
