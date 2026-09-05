//! Exact-target pointer transitions from the ECS UI resolver.
//!
//! These events do not propagate through [`ChildOf`](bevy_ecs::hierarchy::ChildOf): global
//! observers and observers installed for the resolved `entity` are the only observers invoked.
//! The resolver records every transition before this module triggers it, so observer mutations run
//! after selection and input consumption for every layout's pointer pass are final. They can
//! affect later frames, but cannot retarget, consume, or otherwise change the transition being
//! delivered. Forced leaves use the current cursor against a retained `UINode` transform; if that
//! component was removed, they use the last successfully resolved position.

use crate::{input::MouseButton, math::Vec2};
use bevy_ecs::prelude::*;

/// Pointer coordinates captured while resolving a UI pointer transition.
#[derive(Clone, Copy, Debug)]
pub struct UIPointerPosition {
    /// Window/screen coordinates.
    pub screen: Vec2,
    /// Coordinates in the target node.
    pub local: Vec2,
    /// Coordinates in the target node's immediate parent.
    pub parent: Vec2,
}

/// Emitted when a pointer enters a UI node.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct UIPointerEnter {
    pub entity: Entity,
    pub position: UIPointerPosition,
}

/// Emitted when a pointer leaves a UI node.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct UIPointerLeave {
    pub entity: Entity,
    pub position: UIPointerPosition,
}

/// Emitted for an accepted physical pointer-button press.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct UIPointerPressed {
    pub entity: Entity,
    pub button: MouseButton,
    pub position: UIPointerPosition,
}

/// Emitted for an accepted physical pointer-button release.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct UIPointerReleased {
    pub entity: Entity,
    pub button: MouseButton,
    pub position: UIPointerPosition,
}

/// Emitted when a pointer-button release qualifies as a click.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct UIClick {
    pub entity: Entity,
    pub button: MouseButton,
    pub position: UIPointerPosition,
}

/// Emitted when scrolling is resolved to a UI node.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct UIScrollInput {
    pub entity: Entity,
    pub delta: Vec2,
    pub position: UIPointerPosition,
}

/// Emitted for a resolved UI drag lifecycle transition.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct UIDragInput {
    pub entity: Entity,
    pub button: MouseButton,
    pub event: super::components::UIDragEvent,
    pub position: UIPointerPosition,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum ResolvedPointerTransition {
    Enter(UIPointerEnter),
    Leave(UIPointerLeave),
    Pressed(UIPointerPressed),
    Released(UIPointerReleased),
    Click(UIClick),
    Scroll(UIScrollInput),
    Drag(UIDragInput),
}

impl ResolvedPointerTransition {
    fn entity(self) -> Entity {
        match self {
            Self::Enter(event) => event.entity,
            Self::Leave(event) => event.entity,
            Self::Pressed(event) => event.entity,
            Self::Released(event) => event.entity,
            Self::Click(event) => event.entity,
            Self::Scroll(event) => event.entity,
            Self::Drag(event) => event.entity,
        }
    }

    fn trigger(self, world: &mut World) {
        match self {
            Self::Enter(event) => world.trigger(event),
            Self::Leave(event) => world.trigger(event),
            Self::Pressed(event) => world.trigger(event),
            Self::Released(event) => world.trigger(event),
            Self::Click(event) => world.trigger(event),
            Self::Scroll(event) => world.trigger(event),
            Self::Drag(event) => world.trigger(event),
        }
    }
}

#[derive(Resource, Default)]
pub(super) struct ResolvedPointerTransitions(Vec<ResolvedPointerTransition>);

impl ResolvedPointerTransitions {
    pub(super) fn push(&mut self, transition: ResolvedPointerTransition) {
        self.0.push(transition);
    }
}

pub(super) fn begin(mut transitions: ResMut<ResolvedPointerTransitions>) {
    transitions.0.clear();
}

pub(super) fn dispatch(world: &mut World) {
    world.resource_scope(
        |world: &mut World, mut transitions: Mut<ResolvedPointerTransitions>| {
            for transition in transitions.0.drain(..) {
                if world
                    .get::<super::components::UIPointer>(transition.entity())
                    .is_some()
                {
                    transition.trigger(world);
                }
            }
        },
    );
}
