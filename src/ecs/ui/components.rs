use super::layout::UILayoutId;
use crate::draw::*;
use crate::math::Vec2;
use bevy_ecs::prelude::*;
use bevy_ecs::query::ReadOnlyQueryData;
use taffy::prelude::*;

#[derive(Component)]
pub struct UINode<T>
where
    T: Send + Sync + 'static,
{
    pub(super) layout: UILayoutId<T>,
    pub(super) node_id: NodeId,
    pub(super) position: Vec2,
    pub(super) size: Vec2,
}

impl<T> UINode<T>
where
    T: Send + Sync + 'static,
{
    #[inline]
    pub fn layout(&self) -> &UILayoutId<T> {
        &self.layout
    }

    #[inline]
    pub fn position(&self) -> Vec2 {
        self.position
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        self.size
    }
}

#[derive(Component)]
pub struct UIRender {
    pub(super) cb: Box<dyn Fn(&mut Draw2D, &World, Entity) + Send + Sync + 'static>,
}

impl UIRender {
    pub fn new<Q, F>(cb: F) -> Self
    where
        Q: ReadOnlyQueryData + 'static,
        F: Fn(&mut Draw2D, Q::Item<'_>) + Send + Sync + 'static,
    {
        let wrapped = Box::new(move |draw: &mut Draw2D, world: &World, entity: Entity| {
            cb(draw, world.entity(entity).components::<Q>());
        });
        Self { cb: wrapped }
    }

    pub(super) fn render(&self, draw: &mut Draw2D, world: &World, entity: Entity) {
        (self.cb)(draw, world, entity);
    }
}
