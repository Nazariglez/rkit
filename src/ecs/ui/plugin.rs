use super::{
    components::{UIDragEvent, UINode, UIPointer, UIPointerConsumePolicy, UITransform},
    layout::{UILayout, UINodeGraph},
    prelude::{UIImage, UIText},
    style::UIStyle,
};
use crate::{
    ecs::{app::App, input::Mouse, plugin::Plugin, schedules::OnPostUpdate},
    input::MouseButton,
    math::Vec2,
    prelude::OnPreUpdate,
};
use bevy_ecs::prelude::*;
use indexmap::IndexMap;
use strum::IntoEnumIterator;

#[derive(Debug, Event, Clone, Copy)]
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
            .add_event::<UILayoutUpdateEvent<T>>()
            .on_schedule(
                OnPreUpdate,
                pointer_interactivity_system::<T>
                    .run_if(is_layout_present::<T>)
                    .in_set(UILayoutSysSet),
            )
            .on_schedule(
                OnPostUpdate,
                (
                    remove_system::<T>,
                    change_style_system::<T>,
                    update_layout_system::<T>,
                    update_nodes_system::<T>,
                    update_pointer_transform_system::<T>,
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

pub(super) fn update_layout_system<T: Component>(
    mut layout: ResMut<UILayout<T>>,
    mut evt: EventWriter<UILayoutUpdateEvent<T>>,
    images: Query<&UIImage, With<T>>,
    texts: Query<&UIText, With<T>>,
) {
    let updated = layout.update(images, texts);
    if updated {
        evt.send(UILayoutUpdateEvent::<T>::default());
    }
}

#[allow(clippy::type_complexity)]
fn update_nodes_system<T: Component>(
    mut node_query: Query<(&mut UINode, &UIStyle, &UITransform), With<T>>,
    layout: ResMut<UILayout<T>>,
    mut evt: EventReader<UILayoutUpdateEvent<T>>,
) {
    #[derive(Clone, Copy, Hash, PartialEq, Eq)]
    enum EntityId {
        Empty,
        Raw(Entity),
    }

    for _ in evt.read() {
        // update nodes
        node_query
            .iter_mut()
            .for_each(|(mut node, _, _)| layout.set_node_layout(&mut node));

        let mut stack = IndexMap::new();
        stack.insert(EntityId::Empty, (layout.base_transform, 1.0));
        layout.graph.iter().for_each(|ng| match ng {
            UINodeGraph::Begin(entity) => {
                if let Ok((mut node, style, transform)) = node_query.get_mut(*entity) {
                    let (_, (last_transform, last_alpha)) = stack.last().unwrap();
                    let is_hide = matches!(style.display, crate::ecs::ui::style::Display::None);
                    node.global_alpha = if is_hide {
                        0.0
                    } else {
                        last_alpha * style.opacity
                    };
                    node.update_transform(transform, *last_transform);
                    stack.insert(
                        EntityId::Raw(*entity),
                        (node.global_transform, node.global_alpha),
                    );
                }
            }
            UINodeGraph::End(entity) => {
                stack.swap_remove(&EntityId::Raw(*entity));
            }
            _ => {}
        });

        debug_assert!(
            stack.len() == 1,
            "Stack transform must be one but is not. (current len: {})",
            stack.len()
        );
    }
}

#[allow(clippy::type_complexity)]
fn update_pointer_transform_system<T: Component>(
    mut pointer_query: Query<(&UINode, &mut UIPointer), With<T>>,
    layout: Res<UILayout<T>>,
    mut evt: EventReader<UILayoutUpdateEvent<T>>,
) {
    for _ in evt.read() {
        // after update nodes, update transform on pointers
        pointer_query.iter_mut().for_each(|(node, mut pointer)| {
            pointer.inverse_transform =
                node.global_transform.inverse() * layout.cam_info.inverse_transform;
            pointer.parent_inverse_transform =
                node.parent_global_transform.inverse() * layout.cam_info.transform;
        });
    }
}

#[allow(clippy::type_complexity)]
fn remove_system<T: Component>(
    mut layout: ResMut<UILayout<T>>,
    mut removed_nodes: RemovedComponents<UINode>,
    mut removed_layouts: RemovedComponents<T>,
    mut removed_style: RemovedComponents<UIStyle>,
    mut removed_transform: RemovedComponents<UITransform>,
) {
    let iterator = removed_nodes
        .read()
        .chain(removed_layouts.read())
        .chain(removed_style.read())
        .chain(removed_transform.read());

    for entity in iterator {
        layout.remove_node(entity);
    }
}

#[allow(clippy::type_complexity)]
fn change_style_system<T: Component>(
    mut query: Query<
        (&UINode, &UIStyle),
        (
            With<T>,
            Or<(
                Changed<UIStyle>,
                Changed<UITransform>,
                Changed<UIText>,
                Changed<UIImage>,
            )>,
        ),
    >,
    mut layout: ResMut<UILayout<T>>,
) {
    for (node, style) in query.iter_mut() {
        layout.set_node_style(node, style);
    }
}

#[allow(clippy::type_complexity)]
fn pointer_interactivity_system<T: Component>(
    mut query: Query<(&mut UIPointer, &UINode, Option<&UIPointerConsumePolicy>), With<T>>,
    layout: Res<UILayout<T>>,
    mut mouse: ResMut<Mouse>,
) {
    let default_policy = UIPointerConsumePolicy::all();

    let pos = mouse.position();
    let mut consumed_hover = false;
    let mut consumed_click = false;

    let mut down_button = mouse.down_buttons();
    let mut pressed_button = mouse.pressed_buttons();
    let mut released_button = mouse.released_buttons();
    let mut scrolling = mouse.is_scrolling().then_some(mouse.wheel_delta());
    let is_moving = mouse.is_moving();

    layout.graph.iter().rev().for_each(|ng| {
        if let UINodeGraph::Node(entity) = ng {
            if let Ok((mut pointer, node, policy)) = query.get_mut(*entity) {
                let policy = policy.unwrap_or(&default_policy);
                let local_pos = layout
                    .cam_info
                    .screen_to_local(pos, pointer.inverse_transform);
                let parent_pos = layout
                    .cam_info
                    .screen_to_local(pos, pointer.parent_inverse_transform);

                let min = Vec2::ZERO;
                let max = node.size;
                let contains_point = local_pos.x >= min.x
                    && local_pos.y >= min.y
                    && local_pos.x < max.x
                    && local_pos.y < max.y;

                let is_hover = contains_point && !consumed_hover;
                let just_enter = !pointer.is_hover && is_hover;
                let just_exit = pointer.is_hover && !is_hover;

                // consume on_hover for next nodes
                if is_hover && policy.on_hover {
                    consumed_hover = true;
                }

                pointer.position = local_pos;
                pointer.is_hover = is_hover;
                pointer.just_enter = just_enter;
                pointer.just_exit = just_exit;

                // clean last drag events
                pointer.dragging.clear();

                // dragging events
                MouseButton::iter().for_each(|btn| {
                    let init_click = pointer.init_click.contains_key(&btn);
                    let drag_started = pointer.init_drag.contains_key(&btn);
                    let is_down = mouse.is_down(btn);

                    if is_moving {
                        let can_start = init_click && is_down && !drag_started;
                        let can_move = drag_started && is_down;

                        if can_start {
                            let start_pos = pointer.init_click.get(&btn).cloned().unwrap();
                            pointer
                                .init_drag
                                .insert(btn, (start_pos, parent_pos))
                                .unwrap();
                            pointer
                                .dragging
                                .insert(btn, UIDragEvent::Start(parent_pos))
                                .unwrap();
                        } else if can_move {
                            let (start_pos, last_frame_parent_pos) =
                                pointer.init_drag.get(&btn).cloned().unwrap();
                            let delta = parent_pos - last_frame_parent_pos;
                            pointer
                                .dragging
                                .insert(
                                    btn,
                                    UIDragEvent::Move {
                                        start_pos,
                                        current_pos: parent_pos,
                                        delta,
                                    },
                                )
                                .unwrap();
                            pointer
                                .init_drag
                                .insert(btn, (start_pos, parent_pos))
                                .unwrap();
                        }
                    }

                    let can_end = drag_started && !is_down;
                    if can_end {
                        pointer.dragging.remove(&btn);
                        pointer.init_drag.remove(&btn);
                        pointer
                            .dragging
                            .insert(btn, UIDragEvent::End(parent_pos))
                            .unwrap();
                    }
                });

                // clean last frame states
                pointer.down.clear();
                pointer.pressed.clear();
                pointer.released.clear();
                pointer.clicked.clear();
                pointer.scrolling = None;

                // button events
                if is_hover {
                    MouseButton::iter().for_each(|btn| {
                        if down_button.contains(btn) {
                            pointer.down.insert(btn).unwrap();

                            // consume
                            if policy.on_down.contains(&btn) {
                                down_button.remove(btn);
                            }

                            if policy.block_global_down.contains(&btn) {
                                mouse.clear_down_btn(btn);
                            }
                        }

                        if pressed_button.contains(btn) {
                            pointer.pressed.insert(btn).unwrap();
                            pointer.init_click.insert(btn, local_pos).unwrap();

                            // consume
                            if policy.on_pressed.contains(&btn) {
                                pressed_button.remove(btn);
                            }

                            if policy.block_global_pressed.contains(&btn) {
                                mouse.clear_pressed_btn(btn);
                            }
                        }

                        if released_button.contains(btn) {
                            pointer.released.insert(btn).unwrap();

                            if policy.on_released.contains(&btn) {
                                released_button.remove(btn);
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
                    });

                    // scroll
                    pointer.scrolling = scrolling;
                    if scrolling.is_some() && policy.on_scroll {
                        scrolling = None;
                    }
                } else {
                    pointer.init_click.clear();
                }
            }
        }
    });
}
