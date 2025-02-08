use super::components::UINode;
use super::layout::UILayout;
use super::style::UIStyle;
use crate::ecs::app::App;
use crate::ecs::plugin::Plugin;
use crate::ecs::schedules::OnPostUpdate;
use crate::ecs::window::Window;
use bevy_ecs::prelude::*;

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
        let compute_system = generate_compute_system::<T>();
        let remove_system = generate_remove_system::<T>();
        let change_style_system = generate_change_style_system::<T>();
        app.add_resource(UILayout::<T>::default()).add_systems(
            OnPostUpdate,
            (remove_system, change_style_system, compute_system).chain(),
        )
    }
}

// TODO: interactivity and camera support?
fn generate_compute_system<T: Component>(
) -> impl Fn(Query<&mut UINode, With<T>>, ResMut<UILayout<T>>, Res<Window>) {
    |mut query, mut layout, win| {
        layout.set_size(win.size()); // TODO: fixme
        let updated = layout.update();
        if updated {
            query
                .iter_mut()
                .for_each(|mut node| layout.set_node_layout(&mut node));
        }
    }
}

fn generate_remove_system<T: Component>() -> impl Fn(
    ResMut<UILayout<T>>,
    RemovedComponents<UINode>,
    RemovedComponents<T>,
    RemovedComponents<UIStyle>,
) {
    move |mut layout, mut removed_nodes, mut removed_layouts, mut removed_style| {
        let iterator = removed_nodes
            .read()
            .chain(removed_layouts.read())
            .chain(removed_style.read());

        for entity in iterator {
            layout.remove_node(entity);
        }
    }
}

fn generate_change_style_system<T: Component>(
) -> impl Fn(Query<(&UINode, &UIStyle), (With<T>, Changed<UIStyle>)>, ResMut<UILayout<T>>) {
    move |query, mut layout| {
        for (node, style) in query.iter() {
            layout.set_node_style(node, style);
        }
    }
}
