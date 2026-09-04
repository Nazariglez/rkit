use super::{
    components::{UIDragEvent, UINode, UIPointer, UIPointerConsumePolicy, UIScroll, UITransform},
    ctx::UINodeType,
    layout::{
        ManagedUI, UILayout, UILayoutOwner, UILayoutRoot, UINodeGraph, UIProjection,
        UIProjectionField,
    },
    prelude::{UIImage, UIRichText, UIText},
    style::{Display, UIOverflow, UIStyle},
};
use crate::{
    ecs::{app::App, input::Mouse, plugin::Plugin, schedules::OnPostUpdate},
    input::MouseButton,
    input_transition::{ButtonEdge, ordered_button_edges},
    math::{Mat3, Vec2, vec2},
    prelude::{OnPreUpdate, PanicContext},
};
use bevy_ecs::{
    prelude::*,
    system::{RunSystemOnce, SystemParam},
};
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

#[derive(Resource)]
struct UILayoutInstalled<T: Component> {
    root: Entity,
    marker: std::marker::PhantomData<T>,
}
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
        if app.world.contains_resource::<UILayoutInstalled<T>>() {
            return;
        }
        let root = app.world.spawn(UILayoutRoot::<T>::new()).id();
        app.insert_resource(UILayout::<T>::with_root(root))
            .insert_resource(UILayoutInstalled::<T> {
                root,
                marker: std::marker::PhantomData,
            })
            .add_message::<UILayoutUpdateEvent<T>>()
            .on_schedule(
                OnPreUpdate,
                (
                    change_style_system::<T>,
                    sync_projection_system::<T>,
                    update_layout_system::<T>,
                    reset_unprojected_pointers_system::<T>,
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
                    change_style_system::<T>,
                    sync_projection_system::<T>,
                    update_layout_system::<T>,
                    update_presentation_system::<T>,
                )
                    .chain()
                    .run_if(is_layout_present::<T>)
                    .in_set(UILayoutSysSet),
            )
            .configure_sets(OnPreUpdate, UILayoutSysSet)
            .configure_sets(OnPostUpdate, UILayoutSysSet)
            .on_schedule(
                OnPostUpdate,
                cleanup_orphaned_root_system::<T>.run_if(is_layout_absent::<T>),
            );
    }
}

fn is_layout_present<T: Component>(layout: Option<Res<UILayout<T>>>) -> bool {
    layout.is_some()
}

fn is_layout_absent<T: Component>(layout: Option<Res<UILayout<T>>>) -> bool {
    layout.is_none()
}

fn cleanup_orphaned_root_system<T: Component>(
    roots: Query<Entity, With<UILayoutRoot<T>>>,
    mut commands: Commands,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn mouse_is_scrolling(mouse: Res<Mouse>) -> bool {
    mouse.is_scrolling()
}

type ChangedUIProjection<T> = (
    ManagedUI<T>,
    Or<(
        Added<T>,
        Changed<UILayoutOwner>,
        Added<UIStyle>,
        Changed<UINodeType>,
        Added<UINode>,
        Added<UITransform>,
        Changed<Children>,
        Changed<ChildOf>,
    )>,
);

#[derive(SystemParam)]
struct UIProjectionRemovals<'w, 's, T: Component> {
    marker: RemovedComponents<'w, 's, T>,
    owner: RemovedComponents<'w, 's, UILayoutOwner>,
    style: RemovedComponents<'w, 's, UIStyle>,
    typ: RemovedComponents<'w, 's, UINodeType>,
    node: RemovedComponents<'w, 's, UINode>,
    transform: RemovedComponents<'w, 's, UITransform>,
    children: RemovedComponents<'w, 's, Children>,
    parent: RemovedComponents<'w, 's, ChildOf>,
}

fn observe_change<T: Component, C: Component>(
    layout: &mut UILayout<T>,
    entity: Entity,
    component: UIProjectionField,
    value: Option<&Ref<C>>,
) -> bool {
    value.is_some_and(|value| {
        value.is_changed()
            && layout.observe_projection_change(entity, component, value.last_changed())
    })
}

fn observe_removals<T: Component>(
    layout: &mut UILayout<T>,
    entities: impl Iterator<Item = Entity>,
    component: UIProjectionField,
    root: Entity,
    branches: &Query<UIProjection<T>>,
) -> bool {
    let mut removed = false;
    for entity in entities {
        let gone = match branches.get(entity) {
            Ok(branch) if branch.component_tick(component).is_some() => continue,
            Ok(_) => false,
            Err(_) => true,
        };
        if (entity == root || layout.contains(entity))
            && layout.observe_projection_removal(entity, component)
        {
            removed = true;
        }
        if gone {
            layout.forget_projection(entity);
        }
    }
    removed
}

fn sync_projection_system<T: Component>(
    mut layout: ResMut<UILayout<T>>,
    installed: Res<UILayoutInstalled<T>>,
    branches: Query<UIProjection<T>>,
    changed: Query<UIProjection<T>, ChangedUIProjection<T>>,
    owned: Query<(Entity, &UILayoutOwner), ManagedUI<T>>,
    roots: Query<(Option<Ref<Children>>, Option<Ref<ChildOf>>), With<UILayoutRoot<T>>>,
    mut removed: UIProjectionRemovals<T>,
) {
    layout.bind_root(installed.root);
    let root = installed.root;
    let root_present = match roots.get(root) {
        Ok((children, parent)) => {
            let changed = observe_change(
                &mut layout,
                root,
                UIProjectionField::Children,
                children.as_ref(),
            ) | observe_change(
                &mut layout,
                root,
                UIProjectionField::Parent,
                parent.as_ref(),
            );
            if changed {
                layout.mark_topology_dirty();
            }
            parent.is_none()
        }
        Err(_) => false,
    };
    layout.observe_root(root_present);

    for branch in &changed {
        let cached = layout.contains(branch.entity);
        let fields = [
            UIProjectionField::Marker,
            UIProjectionField::Owner,
            UIProjectionField::NodeType,
            UIProjectionField::Children,
            UIProjectionField::Parent,
        ]
        .into_iter()
        .chain(
            [
                UIProjectionField::Style,
                UIProjectionField::Node,
                UIProjectionField::Transform,
            ]
            .into_iter()
            .filter(|_| !cached),
        );
        let mut topology_changed = false;
        for field in fields {
            if let Some(tick) = branch.changed_component_tick(field) {
                topology_changed |= layout.observe_projection_change(branch.entity, field, tick);
            }
        }
        if topology_changed {
            layout.mark_topology_dirty();
        }
    }

    let mut projection_removed = false;
    projection_removed |= observe_removals(
        &mut layout,
        removed.marker.read(),
        UIProjectionField::Marker,
        root,
        &branches,
    );
    projection_removed |= observe_removals(
        &mut layout,
        removed.owner.read(),
        UIProjectionField::Owner,
        root,
        &branches,
    );
    projection_removed |= observe_removals(
        &mut layout,
        removed.style.read(),
        UIProjectionField::Style,
        root,
        &branches,
    );
    projection_removed |= observe_removals(
        &mut layout,
        removed.typ.read(),
        UIProjectionField::NodeType,
        root,
        &branches,
    );
    projection_removed |= observe_removals(
        &mut layout,
        removed.node.read(),
        UIProjectionField::Node,
        root,
        &branches,
    );
    projection_removed |= observe_removals(
        &mut layout,
        removed.transform.read(),
        UIProjectionField::Transform,
        root,
        &branches,
    );
    projection_removed |= observe_removals(
        &mut layout,
        removed.children.read(),
        UIProjectionField::Children,
        root,
        &branches,
    );
    projection_removed |= observe_removals(
        &mut layout,
        removed.parent.read(),
        UIProjectionField::Parent,
        root,
        &branches,
    );
    if projection_removed {
        layout.mark_topology_dirty();
    }

    if layout.project_if_dirty(&branches) {
        layout.report_unreached(&owned);
    }
}

fn update_layout_system<T: Component>(
    mut layout: ResMut<UILayout<T>>,
    mut nodes: Query<(Entity, &mut UINode, Option<&mut UIScroll>), ManagedUI<T>>,
    mut evt: MessageWriter<UILayoutUpdateEvent<T>>,
    images: Query<&UIImage, With<T>>,
    rich_texts: Query<&UIRichText, With<T>>,
    texts: Query<&UIText, With<T>>,
) {
    if !layout.update(images, rich_texts, texts) {
        return;
    }

    for (entity, mut node, scroll) in &mut nodes {
        if !layout.contains(entity) || !layout.set_node_layout(entity, &mut node) {
            continue;
        }
        if let Some(mut scroll) = scroll
            && let Some(height) = layout.scroll_height(entity)
        {
            scroll.set_max_offset(height);
        }
    }
    evt.write(UILayoutUpdateEvent::<T>::default());
}

pub(super) fn layout_root<T: Component>(world: &mut World) -> Option<Entity> {
    let root = world
        .get_resource::<UILayoutInstalled<T>>()
        .map(|installed| installed.root)
        .or_panic("UILayoutPlugin is not installed");
    let root = world
        .get_resource_mut::<UILayout<T>>()
        .or_panic("UILayout resource is missing")
        .bind_root(root);
    if world.get::<UILayoutRoot<T>>(root).is_some() {
        Some(root)
    } else {
        world.resource_mut::<UILayout<T>>().report_missing_root();
        None
    }
}

pub(super) fn refresh_layout<T: Component>(world: &mut World) {
    world
        .get_resource_mut::<UILayout<T>>()
        .unwrap()
        .mark_topology_dirty();
    world
        .run_system_once(sync_projection_system::<T>)
        .or_panic("Synchronizing ECS UI hierarchy");
    world
        .run_system_once(update_layout_system::<T>)
        .or_panic("Updating ECS UI layout");
}

fn reset_unprojected_pointers_system<T: Component>(
    mut layout: ResMut<UILayout<T>>,
    mut pointers: Query<&mut UIPointer>,
) {
    let mut unprojected = layout.take_unprojected();
    unprojected.retain(|entity| {
        pointers
            .get_mut(*entity)
            .is_ok_and(|mut pointer| pointer.reset_lifecycle())
    });
    layout.restore_unprojected(unprojected);
}

#[allow(clippy::type_complexity)]
fn update_presentation_system<T: Component>(
    mut nodes: Query<(&mut UINode, &UIStyle, &UITransform, Option<&UIScroll>), ManagedUI<T>>,
    layout: Res<UILayout<T>>,
    mut stack: Local<Vec<(Mat3, f32)>>,
) {
    stack.clear();
    stack.push((layout.base_transform, 1.0));
    for event in &layout.graph {
        match event {
            UINodeGraph::Begin(entity) => {
                let (parent_transform, parent_alpha) = *stack.last().unwrap();
                let Ok((mut node, style, transform, scroll)) = nodes.get_mut(*entity) else {
                    stack.push((parent_transform, parent_alpha));
                    continue;
                };
                node.global_alpha = if matches!(style.display, Display::None) {
                    0.0
                } else {
                    parent_alpha * style.opacity
                };
                node.update_transform(transform, parent_transform, layout.cam_info.pixel_perfect);
                let child_transform = scroll.map_or(node.global_transform, |scroll| {
                    node.global_transform * Mat3::from_translation(vec2(0.0, -scroll.offset()))
                });
                stack.push((child_transform, node.global_alpha));
            }
            UINodeGraph::End(_) => {
                stack.pop();
            }
            UINodeGraph::Node(_) => {}
        }
    }

    debug_assert_eq!(stack.len(), 1, "UI presentation stack is unbalanced");
}

#[allow(clippy::type_complexity)]
fn change_style_system<T: Component>(
    changed: Query<
        (
            Entity,
            &UIStyle,
            Has<UIScroll>,
            Option<Ref<UIText>>,
            Option<Ref<UIRichText>>,
            Option<Ref<UIImage>>,
        ),
        (
            With<T>,
            With<UILayoutOwner>,
            Or<(
                Changed<UIStyle>,
                Added<UIScroll>,
                Changed<UIText>,
                Changed<UIRichText>,
                Changed<UIImage>,
            )>,
        ),
    >,
    nodes: Query<(Entity, &UIStyle), ManagedUI<T>>,
    mut removed_scrolls: RemovedComponents<UIScroll>,
    mut layout: ResMut<UILayout<T>>,
) {
    for (entity, style, scroll, text, rich_text, image) in &changed {
        layout.set_node_style(entity, style, scroll);
        if let Some(text) = text.filter(|text| text.is_changed()) {
            layout.invalidate_intrinsic(entity, text.last_changed());
        }
        if let Some(rich_text) = rich_text.filter(|text| text.is_changed()) {
            layout.invalidate_intrinsic(entity, rich_text.last_changed());
        }
        if let Some(image) = image.filter(|image| image.is_changed()) {
            layout.invalidate_intrinsic(entity, image.last_changed());
        }
    }
    for entity in removed_scrolls.read() {
        if let Ok((entity, style)) = nodes.get(entity) {
            layout.set_node_style(entity, style, false);
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
            &UILayoutOwner,
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
    let root = layout.root_entity();
    for (owner, mut pointer, _, _, _, _) in &mut query {
        if owner.root == root {
            pointer.scrolling = None;
        }
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
        let Ok((_, pointer, node, style, scroll, _)) = query.get_mut(*entity) else {
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
        let Ok((_, mut pointer, node, style, mut scroll, policy)) = query.get_mut(*entity) else {
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
