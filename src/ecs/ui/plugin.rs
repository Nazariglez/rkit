use super::{
    components::{UIDragEvent, UINode, UIPointer, UIPointerConsumePolicy, UIScroll, UITransform},
    layout::{UILayout, UINodeGraph},
    prelude::{UIImage, UIRichText, UIText},
    style::{Display, UIOverflow, UIStyle},
};
use crate::{
    ecs::{app::App, input::Mouse, plugin::Plugin, schedules::OnPostUpdate},
    input::MouseButton,
    input_transition::{ButtonEdge, ordered_button_edges},
    math::{Mat3, Vec2, vec2},
    prelude::OnPreUpdate,
};
use bevy_ecs::prelude::*;
use indexmap::IndexMap;
use strum::IntoEnumIterator;

#[derive(Debug, Message, Clone, Copy)]
pub struct UILayoutUpdateEvent<T>(std::marker::PhantomData<T>)
where
    T: Component;

impl<T> Default for UILayoutUpdateEvent<T>
where
    T: Component,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(SystemSet, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct UILayoutSysSet;

pub struct UILayoutPlugin<T>(std::marker::PhantomData<T>);
impl<T> Default for UILayoutPlugin<T>
where
    T: Component,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> Plugin for UILayoutPlugin<T>
where
    T: Component,
{
    fn apply(&self, app: &mut App) {
        app.insert_resource(UILayout::<T>::default())
            .add_message::<UILayoutUpdateEvent<T>>()
            .on_schedule(
                OnPreUpdate,
                (
                    update_layout_system::<T>,
                    update_presentation_system::<T>,
                    update_pointer_eligibility_system::<T>,
                    wheel_interactivity_system::<T>,
                    update_presentation_system::<T>.run_if(mouse_is_scrolling),
                    update_pointer_eligibility_system::<T>.run_if(mouse_is_scrolling),
                    pointer_interactivity_system::<T>,
                )
                    .chain()
                    .run_if(is_layout_present::<T>)
                    .in_set(UILayoutSysSet),
            )
            .on_schedule(
                OnPostUpdate,
                (
                    remove_system::<T>,
                    change_style_system::<T>,
                    update_layout_system::<T>,
                    update_presentation_system::<T>,
                )
                    .chain()
                    .run_if(is_layout_present::<T>)
                    .in_set(UILayoutSysSet),
            )
            .configure_sets(OnPreUpdate, UILayoutSysSet)
            .configure_sets(OnPostUpdate, UILayoutSysSet);
    }
}

fn is_layout_present<T: Component>(layout: Option<Res<UILayout<T>>>) -> bool {
    layout.is_some()
}

fn mouse_is_scrolling(mouse: Res<Mouse>) -> bool {
    mouse.is_scrolling()
}

pub(super) fn update_layout_system<T: Component>(
    mut layout: ResMut<UILayout<T>>,
    mut nodes: Query<(&mut UINode, Option<&mut UIScroll>), With<T>>,
    mut evt: MessageWriter<UILayoutUpdateEvent<T>>,
    images: Query<&UIImage, With<T>>,
    rich_texts: Query<&UIRichText, With<T>>,
    texts: Query<&UIText, With<T>>,
) {
    if !layout.update(images, rich_texts, texts) {
        return;
    }

    for (mut node, scroll) in &mut nodes {
        layout.set_node_layout(&mut node);
        if let Some(mut scroll) = scroll {
            scroll.set_max_offset(layout.scroll_height(&node));
        }
    }
    evt.write(UILayoutUpdateEvent::<T>::default());
}

#[allow(clippy::type_complexity)]
fn update_presentation_system<T: Component>(
    mut nodes: Query<(&mut UINode, &UIStyle, &UITransform, Option<&UIScroll>), With<T>>,
    layout: Res<UILayout<T>>,
) {
    #[derive(Clone, Copy, Hash, PartialEq, Eq)]
    enum EntityId {
        Root,
        Node(Entity),
    }

    let mut stack = IndexMap::new();
    stack.insert(EntityId::Root, (layout.base_transform, 1.0));
    for event in &layout.graph {
        match event {
            UINodeGraph::Begin(entity) => {
                let Ok((mut node, style, transform, scroll)) = nodes.get_mut(*entity) else {
                    continue;
                };
                let (_, (parent_transform, parent_alpha)) = stack.last().unwrap();
                node.global_alpha = if matches!(style.display, Display::None) {
                    0.0
                } else {
                    parent_alpha * style.opacity
                };
                node.update_transform(transform, *parent_transform, layout.cam_info.pixel_perfect);
                let child_transform = match scroll {
                    Some(scroll) => {
                        node.global_transform * Mat3::from_translation(vec2(0.0, -scroll.offset()))
                    }
                    None => node.global_transform,
                };
                stack.insert(
                    EntityId::Node(*entity),
                    (child_transform, node.global_alpha),
                );
            }
            UINodeGraph::End(entity) => {
                stack.swap_remove(&EntityId::Node(*entity));
            }
            UINodeGraph::Node(_) => {}
        }
    }

    debug_assert_eq!(stack.len(), 1, "UI presentation stack is unbalanced");
}

#[allow(clippy::type_complexity)]
fn remove_system<T: Component>(
    mut layout: ResMut<UILayout<T>>,
    mut removed_nodes: RemovedComponents<UINode>,
    mut removed_layouts: RemovedComponents<T>,
    mut removed_style: RemovedComponents<UIStyle>,
    mut removed_transform: RemovedComponents<UITransform>,
) {
    let entities = removed_nodes
        .read()
        .chain(removed_layouts.read())
        .chain(removed_style.read())
        .chain(removed_transform.read());

    for entity in entities {
        layout.remove_node(entity);
    }
}

#[allow(clippy::type_complexity)]
fn change_style_system<T: Component>(
    changed: Query<
        (&UINode, &UIStyle, Has<UIScroll>),
        (
            With<T>,
            Or<(
                Changed<UIStyle>,
                Added<UIScroll>,
                Changed<UIText>,
                Changed<UIRichText>,
                Changed<UIImage>,
            )>,
        ),
    >,
    nodes: Query<(&UINode, &UIStyle), With<T>>,
    mut removed_scrolls: RemovedComponents<UIScroll>,
    mut layout: ResMut<UILayout<T>>,
) {
    for (node, style, scroll) in &changed {
        layout.set_node_style(node, style, scroll);
    }
    for entity in removed_scrolls.read() {
        if let Ok((node, style)) = nodes.get(entity) {
            layout.set_node_style(node, style, false);
        }
    }
}

fn point_in_rect(point: Vec2, size: Vec2) -> bool {
    point.x >= 0.0 && point.y >= 0.0 && point.x < size.x && point.y < size.y
}

fn point_in_rounded_rect(point: Vec2, size: Vec2, radius: f32) -> bool {
    if !radius.is_finite() || size.x <= 0.0 || size.y <= 0.0 || !point_in_rect(point, size) {
        return false;
    }
    let radius = radius.max(0.0).min(size.min_element() * 0.5);
    if radius == 0.0
        || (point.x >= radius && point.x < size.x - radius)
        || (point.y >= radius && point.y < size.y - radius)
    {
        return true;
    }

    let center = vec2(
        if point.x < radius {
            radius
        } else {
            size.x - radius
        },
        if point.y < radius {
            radius
        } else {
            size.y - radius
        },
    );
    point.distance_squared(center) <= radius * radius
}

fn point_in_overflow(point: Vec2, size: Vec2, overflow: UIOverflow) -> bool {
    match overflow {
        UIOverflow::Visible => true,
        UIOverflow::Clip => point_in_rect(point, size),
        UIOverflow::Rounded(radius) => point_in_rounded_rect(point, size, radius),
    }
}

#[allow(clippy::type_complexity)]
fn update_pointer_eligibility_system<T: Component>(
    mut nodes: Query<(&UINode, &UIStyle, Has<UIScroll>, Option<&mut UIPointer>), With<T>>,
    layout: Res<UILayout<T>>,
    mouse: Res<Mouse>,
    mut stack: Local<Vec<bool>>,
) {
    stack.clear();
    let mut eligible = true;
    let cursor = mouse.position();

    for event in &layout.graph {
        match event {
            UINodeGraph::Begin(_) => stack.push(eligible),
            UINodeGraph::Node(entity) => {
                let Ok((node, style, scroll, pointer)) = nodes.get_mut(*entity) else {
                    eligible = false;
                    continue;
                };
                let incoming = eligible && node.is_visible();
                if let Some(mut pointer) = pointer {
                    pointer.ancestor_eligible = incoming;
                }
                let overflow = style.effective_overflow(scroll);
                eligible = incoming
                    && point_in_overflow(layout.screen_to_node(cursor, node), node.size, overflow);
            }
            UINodeGraph::End(_) => eligible = stack.pop().unwrap_or(true),
        }
    }
}

fn wheel_target(
    layout: &UILayout<impl Component>,
    cursor: Vec2,
    pointer: &UIPointer,
    node: &UINode,
    style: &UIStyle,
    scroll: bool,
) -> bool {
    if !pointer.ancestor_eligible || !node.is_visible() {
        return false;
    }
    let point = layout.screen_to_node(cursor, node);
    if !point_in_rect(point, node.size) {
        return false;
    }
    match style.effective_overflow(scroll) {
        UIOverflow::Rounded(radius) if scroll => point_in_rounded_rect(point, node.size, radius),
        _ => true,
    }
}

#[allow(clippy::type_complexity)]
fn wheel_interactivity_system<T: Component>(
    mut query: Query<
        (
            &mut UIPointer,
            &UINode,
            &UIStyle,
            Option<&mut UIScroll>,
            Option<&UIPointerConsumePolicy>,
        ),
        With<T>,
    >,
    layout: Res<UILayout<T>>,
    mouse: Res<Mouse>,
) {
    for (mut pointer, _, _, _, _) in &mut query {
        pointer.scrolling = None;
    }
    if !mouse.is_scrolling() {
        return;
    }

    let cursor = mouse.position();
    let delta = mouse.wheel_delta();
    let mut has_scroll_owner = false;
    for event in layout.graph.iter().rev() {
        let UINodeGraph::Node(entity) = event else {
            continue;
        };
        let Ok((pointer, node, style, scroll, _)) = query.get_mut(*entity) else {
            continue;
        };
        let Some(scroll) = scroll.as_deref() else {
            continue;
        };
        if delta.y != 0.0
            && scroll.max_offset() > 0.0
            && wheel_target(&layout, cursor, &pointer, node, style, true)
        {
            has_scroll_owner = true;
            break;
        }
    }

    let mut scrolling = Some(delta);
    for event in layout.graph.iter().rev() {
        let UINodeGraph::Node(entity) = event else {
            continue;
        };
        let Some(current_delta) = scrolling else {
            break;
        };
        let Ok((mut pointer, node, style, mut scroll, policy)) = query.get_mut(*entity) else {
            continue;
        };
        let has_scroll = scroll.is_some();
        if !wheel_target(&layout, cursor, &pointer, node, style, has_scroll) {
            continue;
        }

        pointer.scrolling = Some(current_delta);
        if let Some(scroll) = scroll.as_deref_mut()
            && current_delta.y != 0.0
            && scroll.max_offset() > 0.0
        {
            scroll.apply_wheel(current_delta.y);
            scrolling = None;
            continue;
        }
        if policy.is_some_and(|policy| policy.on_scroll)
            || (!has_scroll_owner && !has_scroll && policy.is_none())
        {
            scrolling = None;
        }
    }
}

#[allow(clippy::type_complexity)]
fn pointer_interactivity_system<T: Component>(
    mut query: Query<(&mut UIPointer, &UINode, Option<&UIPointerConsumePolicy>), With<T>>,
    layout: Res<UILayout<T>>,
    mut mouse: ResMut<Mouse>,
) {
    let default_policy = UIPointerConsumePolicy::all();
    let cursor = mouse.position();
    let mut consumed_hover = false;
    let mut consumed_click = false;

    let mut down_buttons = mouse.down_buttons();
    let down_for_lifecycle = down_buttons.clone();
    let mut pressed_buttons = mouse.pressed_buttons();
    let mut released_buttons = mouse.released_buttons();
    let released_for_lifecycle = released_buttons.clone();
    let is_moving = mouse.is_moving();

    for event in layout.graph.iter().rev() {
        let UINodeGraph::Node(entity) = event else {
            continue;
        };
        let Ok((mut pointer, node, policy)) = query.get_mut(*entity) else {
            continue;
        };
        let policy = policy.unwrap_or(&default_policy);
        let local_pos = layout.screen_to_node(cursor, node);
        let parent_inverse = node.parent_global_transform.inverse() * layout.cam_info.transform;
        let parent_pos = layout.cam_info.screen_to_local(cursor, parent_inverse);
        let target_eligible =
            pointer.ancestor_eligible && node.is_visible() && point_in_rect(local_pos, node.size);
        let is_hover = target_eligible && !consumed_hover;
        let just_enter = !pointer.is_hover && is_hover;
        let just_exit = pointer.is_hover && !is_hover;

        if is_hover && policy.on_hover {
            consumed_hover = true;
        }
        pointer.position = local_pos;
        pointer.is_hover = is_hover;
        pointer.just_enter = just_enter;
        pointer.just_exit = just_exit;
        pointer.dragging.clear();

        for btn in MouseButton::iter() {
            let init_click = pointer.init_click.contains_key(&btn);
            let drag_started = pointer.init_drag.contains_key(&btn);
            let is_down = mouse.is_down(btn);
            let released = released_for_lifecycle.contains(btn);

            if drag_started && (released || !is_down) {
                pointer.init_drag.remove(&btn);
                pointer
                    .dragging
                    .insert(btn, UIDragEvent::End(parent_pos))
                    .unwrap();
            } else if is_moving && !released {
                let can_start = init_click && is_down && !drag_started && is_hover;
                let can_move = drag_started && is_down;
                if can_start {
                    let start_pos = pointer.init_click.get(&btn).copied().unwrap();
                    pointer
                        .init_drag
                        .insert(btn, (start_pos, parent_pos))
                        .unwrap();
                    pointer
                        .dragging
                        .insert(btn, UIDragEvent::Start(parent_pos))
                        .unwrap();
                } else if can_move {
                    let (start_pos, previous_pos) = pointer.init_drag.get(&btn).copied().unwrap();
                    pointer
                        .dragging
                        .insert(
                            btn,
                            UIDragEvent::Move {
                                start_pos,
                                current_pos: parent_pos,
                                delta: parent_pos - previous_pos,
                            },
                        )
                        .unwrap();
                    pointer
                        .init_drag
                        .insert(btn, (start_pos, parent_pos))
                        .unwrap();
                }
            }
        }

        pointer.down.clear();
        pointer.pressed.clear();
        pointer.released.clear();
        pointer.clicked.clear();

        if is_hover {
            for btn in MouseButton::iter() {
                if down_buttons.contains(btn) {
                    pointer.down.insert(btn).unwrap();
                    if policy.on_down.contains(&btn) {
                        down_buttons.remove(btn);
                    }
                    if policy.block_global_down.contains(&btn) {
                        mouse.clear_down_btn(btn);
                    }
                }

                let pressed = pressed_buttons.contains(btn);
                let released = released_buttons.contains(btn);
                let down = down_for_lifecycle.contains(btn);
                for edge in ordered_button_edges(pressed, released, down) {
                    match edge {
                        ButtonEdge::Pressed => {
                            pointer.pressed.insert(btn).unwrap();
                            pointer.init_click.insert(btn, local_pos).unwrap();
                            if policy.on_pressed.contains(&btn) {
                                pressed_buttons.remove(btn);
                            }
                            if policy.block_global_pressed.contains(&btn) {
                                mouse.clear_pressed_btn(btn);
                            }
                        }
                        ButtonEdge::Released => {
                            pointer.released.insert(btn).unwrap();
                            if policy.on_released.contains(&btn) {
                                released_buttons.remove(btn);
                            }
                            if policy.block_global_released.contains(&btn) {
                                mouse.clear_released_btn(btn);
                            }
                            if pointer.init_click.contains_key(&btn) && !consumed_click {
                                pointer.clicked.insert(btn).unwrap();
                                if policy.on_click.contains(&btn) {
                                    consumed_click = true;
                                }
                            }
                            pointer.init_click.remove(&btn);
                        }
                    }
                }
            }
        } else {
            pointer.init_click.clear();
        }
    }
}
