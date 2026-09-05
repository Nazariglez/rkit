use crate::math::Vec2;
use bevy_ecs::{
    prelude::*,
    query::{ReadOnlyQueryData, ReleaseStateQueryData},
};
use std::sync::atomic::{AtomicU64, Ordering};

/// Space available to an intrinsic UI measurement.
#[derive(Clone, Copy, Debug)]
pub enum UIAvailableSpace {
    /// A concrete available size in logical UI coordinates.
    Definite(f32),
    /// The minimum-content constraint.
    MinContent,
    /// The maximum-content constraint.
    MaxContent,
}

/// Constraints supplied to an intrinsic UI measurement.
#[derive(Clone, Copy, Debug)]
pub struct UIMeasureInput {
    /// Content width fixed for this measurement pass, if any.
    pub known_width: Option<f32>,
    /// Content height fixed for this measurement pass, if any.
    pub known_height: Option<f32>,
    /// Width available to an unconstrained measurement.
    pub available_width: UIAvailableSpace,
    /// Height available to an unconstrained measurement.
    pub available_height: UIAvailableSpace,
}

type MeasureCb = dyn Fn(UIMeasureInput, &World, Entity) -> Option<Vec2> + Send + Sync + 'static;

static NEXT_MEASURE_REVISION: AtomicU64 = AtomicU64::new(1);

fn next_revision() -> u64 {
    NEXT_MEASURE_REVISION.fetch_add(1, Ordering::Relaxed)
}

/// Provides intrinsic size for a UI node from read-only entity data.
///
/// Layout invokes the callback only during a dirty layout update and only when intrinsic content
/// is needed for an axis. Known content axes always win over the callback result. A parent may
/// still fix the node's outer size while Taffy requests intrinsic content for overflow. Call
/// [`Self::invalidate`] after changing queried data that affects size; paint-only changes do not
/// need invalidation. A missing query match measures as zero. Invalid consumed dimensions measure
/// as zero and report a runtime diagnostic.
#[derive(Component)]
pub struct UIMeasure {
    cb: Box<MeasureCb>,
    revision: u64,
}

impl UIMeasure {
    /// Creates a measurement callback from read-only data on the same entity.
    ///
    /// The callback receives semantic UI constraints only; Taffy types are not exposed.
    pub fn run<Q, F>(callback: F) -> Self
    where
        Q: ReadOnlyQueryData + ReleaseStateQueryData + 'static,
        F: Fn(UIMeasureInput, Q::Item<'_, '_>) -> Vec2 + Send + Sync + 'static,
    {
        let cb = Box::new(
            move |input: UIMeasureInput, world: &World, entity: Entity| {
                world
                    .get_entity(entity)
                    .ok()?
                    .get_components::<Q>()
                    .ok()
                    .map(|components| callback(input, components))
            },
        );
        Self {
            cb,
            revision: next_revision(),
        }
    }

    /// Marks this measurement stale after its queried size-affecting data changes.
    pub fn invalidate(&mut self) {
        self.revision = next_revision();
    }

    pub(super) fn measure(
        &self,
        input: UIMeasureInput,
        world: &World,
        entity: Entity,
    ) -> Option<Vec2> {
        (self.cb)(input, world, entity)
    }

    pub(super) fn revision(&self) -> u64 {
        self.revision
    }
}
