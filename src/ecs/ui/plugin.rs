use super::components::UINode;
use super::layout::{UILayout, UINodeGraph};
use super::prelude::{UIPointer, UITransform};
use super::style::UIStyle;
use crate::ecs::app::App;
use crate::ecs::input::Mouse;
use crate::ecs::plugin::Plugin;
use crate::ecs::schedules::OnPostUpdate;
use crate::ecs::window::Window;
use bevy_ecs::prelude::*;
use corelib::math::Mat3;

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
        let compute_system = generate_update_layout_system::<T>();
        let remove_system = generate_remove_system::<T>();
        let change_style_system = generate_change_style_system::<T>();
        let pointer_interactivity_system = generate_pointer_interactivity_system::<T>();
        app.add_resource(UILayout::<T>::default())
            .add_systems(
                OnPostUpdate,
                (remove_system, change_style_system, compute_system)
                    .chain()
                    .in_set(UILayoutSysSet),
            )
            .configure_sets(OnPostUpdate, UILayoutSysSet)
    }
}

fn generate_update_layout_system<T: Component>(
) -> impl Fn(Query<(&mut UINode, &UIStyle, &UITransform), With<T>>, ResMut<UILayout<T>>, Res<Window>)
{
    |mut query, mut layout, win| {
        layout.set_size(win.size()); // TODO: fixme
        let updated = layout.update();
        if updated {
            query
                .iter_mut()
                .for_each(|(mut node, _, _)| layout.set_node_layout(&mut node));

            let mut stack = vec![(Mat3::IDENTITY, 1.0)];
            layout.graph.iter().for_each(|ng| match ng {
                UINodeGraph::Begin(entity) => {
                    if let Ok((mut node, style, transform)) = query.get_mut(*entity) {
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
    Query<(&mut UINode, &UIStyle), (With<T>, Or<(Changed<UIStyle>, Changed<UITransform>)>)>,
    ResMut<UILayout<T>>,
) {
    move |mut query, mut layout| {
        for (mut node, style) in query.iter_mut() {
            node.local_dirty = true;
            layout.set_node_style(node.as_ref(), style);
        }
    }
}

fn generate_pointer_interactivity_system<T: Component>(
) -> impl Fn(Query<(&mut UIPointer, &UINode), With<T>>, Res<UILayout<T>>, Res<Mouse>) {
    move |mut query, layout, mouse| {
        for (mut pointer, node) in &mut query {}
    }
}
