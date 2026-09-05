use crate::draw::*;
use crate::input::MouseButton;
use crate::math::{Mat3, Rect, Vec2};
use bevy_ecs::prelude::*;
use bevy_ecs::query::{ReadOnlyQueryData, ReleaseStateQueryData};
use heapless::index_map::FnvIndexMap;
use heapless::index_set::FnvIndexSet;
use strum::{EnumCount, IntoEnumIterator};

/// The Node contains layout info, as position, size, etc...
#[derive(Component, Clone, Copy, Debug)]
#[require(UITransform)]
pub struct UINode {
    pub(super) position: Vec2,
    pub(super) size: Vec2,

    pub(super) local_transform: Mat3,
    pub(super) global_transform: Mat3,

    pub(super) global_alpha: f32,
}

impl UINode {
    pub(super) fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            size: Vec2::ONE,
            local_transform: Mat3::IDENTITY,
            global_transform: Mat3::IDENTITY,
            global_alpha: 1.0,
        }
    }

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

    #[inline]
    pub fn bounds(&self) -> Rect {
        Rect::new(self.position(), self.size())
    }

    #[inline]
    pub fn is_visible(&self) -> bool {
        self.global_alpha() > 0.0
    }

    pub(super) fn update_transform(
        &mut self,
        transform: &UITransform,
        parent: Mat3,
        pixel_perfect: bool,
    ) {
        let pivot_offset = if pixel_perfect {
            (transform.pivot * self.size).round()
        } else {
            transform.pivot * self.size
        };

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

type RenderCb = dyn Fn(&mut Draw2D, &World, Entity) + Send + Sync + 'static;

/// Draws one UI node through the shared UI traversal.
///
/// [`draw_ui_layout`](super::layout::draw_ui_layout) invokes the callback once while the node's
/// global transform and inherited alpha are active. Coordinates are local to the node; ancestor
/// clips apply, while this node's clip applies to its children. Query data is read-only and comes
/// from the same entity. A missing query match skips rendering. One entity has one `UIRender`;
/// compose visual layers with children or one callback. The callback records [`Draw2D`] commands
/// only: camera selection and submission remain the caller's responsibility.
#[derive(Component)]
pub struct UIRender {
    pub(super) cb: Box<RenderCb>,
}

impl UIRender {
    /// Creates a render callback from read-only data on the same entity.
    pub fn run<Q, F>(cb: F) -> Self
    where
        Q: ReadOnlyQueryData + ReleaseStateQueryData + 'static,
        F: Fn(&mut Draw2D, Q::Item<'_, '_>) + Send + Sync + 'static,
    {
        let wrapped = Box::new(move |draw: &mut Draw2D, world: &World, entity: Entity| {
            let Some(components) = world
                .get_entity(entity)
                .ok()
                .and_then(|entity| entity.get_components::<Q>().ok())
            else {
                return;
            };
            cb(draw, components);
        });
        Self { cb: wrapped }
    }

    pub(super) fn render(&self, draw: &mut Draw2D, world: &World, entity: Entity) {
        (self.cb)(draw, world, entity);
    }
}

/// Dragging events, positions are always the node's parent position
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UIDragEvent {
    Start(Vec2),
    Move {
        start_pos: Vec2,
        current_pos: Vec2,
        delta: Vec2,
    },
    End(Vec2),
}

type MouseButtonSet = FnvIndexSet<MouseButton, { MouseButton::COUNT.next_power_of_two() }>;
type MouseButtonMap<T> = FnvIndexMap<MouseButton, T, { MouseButton::COUNT.next_power_of_two() }>;

/// Defines which pointer events should be consumed by the node
#[derive(Component, Default, Clone, Debug)]
pub struct UIPointerConsumePolicy {
    /// Consume the click event for these buttons.
    pub on_click: MouseButtonSet,
    /// Consume the down event for these buttons.
    pub on_down: MouseButtonSet,
    /// Consume the pressed event for these buttons.
    pub on_pressed: MouseButtonSet,
    /// Consume the released event for these buttons.
    pub on_released: MouseButtonSet,
    /// Consume the scroll event.
    pub on_scroll: bool,
    /// Consume the hover event.
    pub on_hover: bool,

    /// Block the global down event for these buttons.
    pub block_global_down: MouseButtonSet,
    /// Block the global pressed event for these buttons.
    pub block_global_pressed: MouseButtonSet,
    /// Block the global released event for these buttons.
    pub block_global_released: MouseButtonSet,
}

impl UIPointerConsumePolicy {
    pub fn all() -> Self {
        Self {
            on_click: MouseButtonSet::from_iter(MouseButton::iter()),
            on_down: MouseButtonSet::from_iter(MouseButton::iter()),
            on_pressed: MouseButtonSet::from_iter(MouseButton::iter()),
            on_released: MouseButtonSet::from_iter(MouseButton::iter()),
            on_scroll: true,
            on_hover: true,
            block_global_down: MouseButtonSet::from_iter(MouseButton::iter()),
            block_global_pressed: MouseButtonSet::from_iter(MouseButton::iter()),
            block_global_released: MouseButtonSet::from_iter(MouseButton::iter()),
        }
    }

    pub fn none() -> Self {
        Self::default()
    }

    pub fn only_ui() -> Self {
        Self {
            on_click: MouseButtonSet::from_iter(MouseButton::iter()),
            on_down: MouseButtonSet::from_iter(MouseButton::iter()),
            on_pressed: MouseButtonSet::from_iter(MouseButton::iter()),
            on_released: MouseButtonSet::from_iter(MouseButton::iter()),
            on_scroll: true,
            on_hover: true,
            ..Default::default()
        }
    }

    pub fn only_global() -> Self {
        Self {
            block_global_down: MouseButtonSet::from_iter(MouseButton::iter()),
            block_global_pressed: MouseButtonSet::from_iter(MouseButton::iter()),
            block_global_released: MouseButtonSet::from_iter(MouseButton::iter()),
            ..Default::default()
        }
    }
}

/// Enable Mouse interactivity for the node/entity
#[derive(Component, Default, Clone, Debug)]
pub struct UIPointer {
    position: Option<super::events::UIPointerPosition>,
    pub(super) is_hover: bool,
    pub(super) just_enter: bool,
    pub(super) just_exit: bool,
    pub(super) down: MouseButtonSet,
    pub(super) pressed: MouseButtonSet,
    pub(super) released: MouseButtonSet,
    pub(super) init_click: MouseButtonMap<Vec2>,
    pub(super) init_drag: MouseButtonMap<(Vec2, Vec2)>, // start, and current pos
    pub(super) clicked: MouseButtonSet,
    pub(super) scrolling: Option<Vec2>,
    pub(super) dragging: MouseButtonMap<UIDragEvent>,
    pub(super) ancestor_eligible: bool,
}

impl UIPointer {
    pub fn position(&self) -> Vec2 {
        self.position.map_or(Vec2::ZERO, |position| position.local)
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

    pub fn scroll(&self) -> Option<Vec2> {
        self.scrolling
    }

    pub fn dragging(&self, btn: MouseButton) -> Option<UIDragEvent> {
        self.dragging.get(&btn).cloned()
    }

    pub(super) fn set_position(&mut self, position: super::events::UIPointerPosition) {
        self.position = Some(position);
    }

    pub(super) fn last_position(&self) -> Option<super::events::UIPointerPosition> {
        self.position
    }

    pub(super) fn reset_lifecycle(&mut self) -> bool {
        self.just_exit = self.is_hover;
        self.is_hover = false;
        self.just_enter = false;
        self.down.clear();
        self.pressed.clear();
        self.released.clear();
        self.clicked.clear();
        self.init_click.clear();
        self.init_drag.clear();
        self.dragging.clear();
        self.scrolling = None;
        self.ancestor_eligible = false;
        self.just_exit
    }
}

#[derive(Component, Default, Clone, Copy, Debug)]
#[require(UIPointer)]
pub struct UIScroll {
    offset: f32,
    max_offset: f32,
}

impl UIScroll {
    pub fn vertical() -> Self {
        Self::default()
    }

    pub fn offset(&self) -> f32 {
        self.offset
    }

    pub fn set_offset(&mut self, offset: f32) {
        if offset.is_finite() {
            self.offset = offset.clamp(0.0, self.max_offset);
        }
    }

    pub fn scroll_to(&mut self, offset: f32) {
        self.set_offset(offset);
    }

    pub fn max_offset(&self) -> f32 {
        self.max_offset
    }

    pub(super) fn set_max_offset(&mut self, max_offset: f32) {
        self.max_offset = if max_offset.is_finite() {
            max_offset.max(0.0)
        } else {
            0.0
        };
        self.offset = self.offset.clamp(0.0, self.max_offset);
    }

    pub(super) fn apply_wheel(&mut self, delta: f32) {
        self.set_offset(self.offset - delta);
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
