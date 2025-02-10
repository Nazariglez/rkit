use crate::draw::*;
use crate::input::MouseButton;
use crate::math::{Mat3, Vec2};
use crate::utils::next_pot2;
use bevy_ecs::prelude::*;
use bevy_ecs::query::ReadOnlyQueryData;
use heapless::{FnvIndexMap, FnvIndexSet};
use strum::EnumCount;
use taffy::prelude::*;

/// The Node contains layout info, as position, size, etc...
#[derive(Component)]
#[require(UITransform)]
pub struct UINode {
    pub(super) node_id: NodeId,
    pub(super) position: Vec2,
    pub(super) size: Vec2,

    pub(super) local_transform: Mat3,
    pub(super) global_transform: Mat3,

    pub(super) global_alpha: f32,
}

impl UINode {
    #[inline]
    pub fn position(&self) -> Vec2 {
        self.position
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        self.size
    }

    #[inline]
    pub fn local_transform(&self) -> Mat3 {
        self.local_transform
    }

    #[inline]
    pub fn global_transform(&self) -> Mat3 {
        self.global_transform
    }

    #[inline]
    pub fn global_alpha(&self) -> f32 {
        self.global_alpha
    }

    pub(super) fn update_transform(&mut self, transform: &UITransform, parent: Mat3) {
        let pivot_offset = transform.pivot * self.size;
        let position = self.position + transform.offset;

        let translate = Mat3::from_translation(position);
        let pivot_translate = Mat3::from_translation(pivot_offset);
        let rotation = Mat3::from_angle(transform.rotation);
        let scale = Mat3::from_scale(transform.scale);
        let pivot_translate_back = Mat3::from_translation(-pivot_offset);

        self.local_transform =
            translate * pivot_translate * rotation * scale * pivot_translate_back;

        self.global_transform = parent * self.local_transform;
    }
}

/// Wrapper for the render callback
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

type MouseButtonSet = FnvIndexSet<MouseButton, { next_pot2(MouseButton::COUNT) }>;
type MouseButtonMap<T> = FnvIndexMap<MouseButton, T, { next_pot2(MouseButton::COUNT) }>;

/// Enable Mouse interactivity for the node/entity
#[derive(Component, Default, Clone)]
pub struct UIPointer {
    pub(super) position: Vec2,
    pub(super) is_hover: bool,
    pub(super) just_enter: bool,
    pub(super) just_exit: bool,
    pub(super) down: MouseButtonSet,
    pub(super) pressed: MouseButtonSet,
    pub(super) released: MouseButtonSet,
    pub(super) clicked: MouseButtonSet,
    pub(super) scrolling: Option<Vec2>,
    pub(super) dragging: MouseButtonMap<Vec2>,

    pub(super) inverse_transform: Mat3,
}

impl UIPointer {
    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn is_hover(&self) -> bool {
        self.is_hover
    }

    pub fn just_enter(&self) -> bool {
        self.just_enter
    }

    pub fn just_exit(&self) -> bool {
        self.just_exit
    }

    pub fn is_down(&self, btn: MouseButton) -> bool {
        self.down.contains(&btn)
    }

    pub fn just_pressed(&self, btn: MouseButton) -> bool {
        self.pressed.contains(&btn)
    }

    pub fn just_released(&self, btn: MouseButton) -> bool {
        self.released.contains(&btn)
    }

    pub fn just_clicked(&self, btn: MouseButton) -> bool {
        self.clicked.contains(&btn)
    }

    pub fn scroll(&self) -> Vec2 {
        self.scrolling.unwrap_or_default()
    }

    pub fn dragging(&self, btn: MouseButton) -> Option<Vec2> {
        self.dragging.get(&btn).copied()
    }
}

#[derive(Component)]
pub struct UITransform {
    pub offset: Vec2,
    pub pivot: Vec2,
    pub scale: Vec2,
    pub rotation: f32,
}

impl Default for UITransform {
    fn default() -> Self {
        Self {
            offset: Default::default(),
            pivot: Vec2::splat(0.5),
            scale: Vec2::ONE,
            rotation: Default::default(),
        }
    }
}
