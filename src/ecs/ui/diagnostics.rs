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

/// Reports recoverable failures from the experimental ECS UI API.
#[derive(Event, Clone, Debug)]
#[non_exhaustive]
pub enum UIRuntimeError {
    InvalidHierarchy {
        entity: Entity,
        reason: UIHierarchyError,
    },
    InvalidMeasure {
        entity: Entity,
        size: Vec2,
    },
}
