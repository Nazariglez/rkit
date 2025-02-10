use super::components::UINode;
use super::layout::{UILayout, UINodeGraph};
use super::prelude::{UIPointer, UITransform};
use super::style::UIStyle;
use crate::ecs::app::App;
use crate::ecs::input::Mouse;
use crate::ecs::plugin::Plugin;
use crate::ecs::schedules::OnPostUpdate;
use crate::prelude::OnPreUpdate;
use bevy_ecs::prelude::*;
use corelib::math::{Mat3, Vec2};

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
    fn apply(self, app: App) -> App {
        let update_layout_system = generate_update_layout_system::<T>();
        let update_nodes_system = generate_update_node_system::<T>();
        let update_pointer_transform_system = generate_update_pointer_transform_system::<T>();
        let remove_system = generate_remove_system::<T>();
        let change_style_system = generate_change_style_system::<T>();
        let pointer_interactivity_system = generate_pointer_interactivity_system::<T>();
        app.add_resource(UILayout::<T>::default())
            .add_event::<UILayoutUpdateEvent<T>>()
            .add_systems(
                OnPreUpdate,
                pointer_interactivity_system.in_set(UILayoutSysSet),
            )
            .add_systems(
                OnPostUpdate,
                (
                    remove_system,
                    change_style_system,
                    update_layout_system,
                    update_nodes_system,
                    update_pointer_transform_system,
                )
                    .chain()
                    .in_set(UILayoutSysSet),
            )
            .configure_sets(OnPreUpdate, UILayoutSysSet)
            .configure_sets(OnPostUpdate, UILayoutSysSet)
    }
}

fn generate_update_layout_system<T: Component>(
) -> impl Fn(ResMut<UILayout<T>>, EventWriter<UILayoutUpdateEvent<T>>) {
    |mut layout, mut evt| {
        let updated = layout.update();
        if updated {
            evt.send(UILayoutUpdateEvent::<T>::default());
        }
    }
}

fn generate_update_node_system<T: Component>() -> impl Fn(
    Query<(&mut UINode, &UIStyle, &UITransform), With<T>>,
    ResMut<UILayout<T>>,
    EventReader<UILayoutUpdateEvent<T>>,
) {
    move |mut node_query, layout, mut evt| {
        for _ in evt.read() {
            // update nodes
            node_query
                .iter_mut()
                .for_each(|(mut node, _, _)| layout.set_node_layout(&mut node));

            let mut stack = vec![(Mat3::IDENTITY, 1.0)];
            layout.graph.iter().for_each(|ng| match ng {
                UINodeGraph::Begin(entity) => {
                    if let Ok((mut node, style, transform)) = node_query.get_mut(*entity) {
                        let (last_transform, last_alpha) = stack.last().copied().unwrap();
                        node.global_alpha = last_alpha * style.opacity;
                        node.update_transform(transform, last_transform);
                        stack.push((node.global_transform, node.global_alpha));
                    }
                }
                UINodeGraph::End(_entity) => {
                    stack.pop();
                }
                _ => {}
            });

            debug_assert!(
                stack.len() == 1,
                "Stack transform msut be one but is not {}",
                stack.len()
            );
        }
    }
}

fn generate_update_pointer_transform_system<T: Component>(
) -> impl Fn(Query<(&UINode, &mut UIPointer), With<T>>, EventReader<UILayoutUpdateEvent<T>>) {
    move |mut pointer_query, mut evt| {
        for _ in evt.read() {
            // after update nodes, update transform on pointers
            pointer_query.iter_mut().for_each(|(node, mut pointer)| {
                pointer.inverse_transform = node.global_transform.inverse();
            });
        }
    }
}

fn generate_remove_system<T: Component>() -> impl Fn(
    ResMut<UILayout<T>>,
    RemovedComponents<UINode>,
    RemovedComponents<T>,
    RemovedComponents<UIStyle>,
    RemovedComponents<UITransform>,
) {
    move |mut layout,
          mut removed_nodes,
          mut removed_layouts,
          mut removed_style,
          mut removed_transform| {
        let iterator = removed_nodes
            .read()
            .chain(removed_layouts.read())
            .chain(removed_style.read())
            .chain(removed_transform.read());

        for entity in iterator {
            layout.remove_node(entity);
        }
    }
}

fn generate_change_style_system<T: Component>() -> impl Fn(
    Query<(&UINode, &UIStyle), (With<T>, Or<(Changed<UIStyle>, Changed<UITransform>)>)>,
    ResMut<UILayout<T>>,
) {
    move |mut query, mut layout| {
        for (node, style) in query.iter_mut() {
            layout.set_node_style(node, style);
        }
    }
}

fn generate_pointer_interactivity_system<T: Component>(
) -> impl Fn(Query<(&mut UIPointer, &UINode), With<T>>, Res<UILayout<T>>, Res<Mouse>) {
    move |mut query, layout, mouse| {
        let pos = mouse.position();
        let mut down_button = mouse.down_buttons();
        let mut pressed_button = mouse.pressed_buttons();
        let mut released_button = mouse.released_buttons();
        let mut scrolling = mouse.is_scrolling().then_some(mouse.wheel_delta());

        layout.graph.iter().rev().for_each(|ng| {
            if let UINodeGraph::Node(entity) = ng {
                if let Ok((mut pointer, node)) = query.get_mut(*entity) {
                    let local_pos = layout
                        .cam_info
                        .screen_to_local(pos, pointer.inverse_transform);

                    let min = Vec2::ZERO;
                    let max = node.size;
                    let is_hover = local_pos.x >= min.x
                        && local_pos.y >= min.y
                        && local_pos.x < max.x
                        && local_pos.y < max.y;

                    let just_enter = !pointer.is_hover && is_hover;
                    let just_exit = pointer.is_hover && !is_hover;

                    pointer.is_hover = is_hover;
                    pointer.just_enter = just_enter;
                    pointer.just_exit = just_exit;

                    // TODO: reset storing button, store start click, rekease will release everything etc.. check old ui
                    // manager
                }
            }
        });
    }
}
