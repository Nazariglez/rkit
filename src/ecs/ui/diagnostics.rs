use crate::math::Vec2;
use bevy_ecs::prelude::*;

/// The reason an ECS UI hierarchy branch was excluded from layout.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum UIHierarchyError {
    MissingRoot,
    MissingRequiredComponent,
    WrongLayout,
    WrongParent,
    Cycle,
}

/// The reason an experimental ECS UI scene was rejected before materialization.
#[cfg(feature = "ecs-ui-experimental")]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum UISceneError {
    MissingEntity,
    MissingParent,
    WrongLayout,
    RuntimeOwnedComponent,
}

/// Reports recoverable failures from the experimental ECS UI API.
#[derive(Event, Clone, Debug)]
#[non_exhaustive]
pub enum UIRuntimeError {
    #[cfg(feature = "ecs-ui-experimental")]
    MissingLayout {
        layout: &'static str,
    },
    #[cfg(feature = "ecs-ui-experimental")]
    InvalidScene {
        root: Entity,
        reason: UISceneError,
    },
    InvalidHierarchy {
        entity: Entity,
        reason: UIHierarchyError,
    },
    InvalidMeasure {
        entity: Entity,
        size: Vec2,
    },
}
