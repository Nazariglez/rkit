use super::{
    components::{UIDragEvent, UINode, UIPointer, UIPointerConsumePolicy, UIScroll, UITransform},
    diagnostics::UIRuntimeError,
    events::{
        self, ResolvedPointerTransition, ResolvedPointerTransitions, UIClick, UIDragInput,
        UIPointerEnter, UIPointerLeave, UIPointerPosition, UIPointerPressed, UIPointerReleased,
        UIScrollInput,
    },
    layout::{
        ManagedUI, UIIntrinsicSource, UILayout, UILayoutOwner, UILayoutRoot, UINodeGraph,
        UIProjection, UIProjectionField, valid_layout_root,
    },
    measure::UIMeasure,
    style::{Display, UIOverflow, UIStyle},
    widgets::{UIImage, UIRichText, UIText},
};
use crate::{
    ecs::{app::App, input::Mouse, plugin::Plugin, schedules::OnPostUpdate},
    input::MouseButton,
    input_transition::{ButtonEdge, ordered_button_edges},
    math::{Mat3, Vec2, vec2},
    prelude::{OnPreUpdate, PanicContext},
};
use bevy_ecs::{
    entity_disabling::Disabled,
    prelude::*,
    query::{Allow, QueryData},
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

#[derive(SystemSet, Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct UILayoutResolveSysSet;

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
        if !app.world.contains_resource::<ResolvedPointerTransitions>() {
            app.insert_resource(ResolvedPointerTransitions::default())
                .on_schedule(
                    OnPreUpdate,
                    events::begin
                        .before(UILayoutResolveSysSet)
                        .in_set(UILayoutSysSet),
                )
                .on_schedule(
                    OnPreUpdate,
                    events::dispatch
                        .after(UILayoutResolveSysSet)
                        .in_set(UILayoutSysSet),
                )
                .configure_sets(OnPreUpdate, UILayoutResolveSysSet.in_set(UILayoutSysSet));
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
                    sync_intrinsic_system::<T>,
                    sync_projection_system::<T>,
                    flush_hierarchy_errors_system::<T>,
                    update_layout_system::<T>,
                    update_presentation_system::<T>,
                    reset_unprojected_pointers_system::<T>,
                    update_pointer_eligibility_system::<T>,
                    wheel_interactivity_system::<T>,
                    update_presentation_system::<T>.run_if(mouse_is_scrolling),
                    update_pointer_eligibility_system::<T>.run_if(mouse_is_scrolling),
                    pointer_interactivity_system::<T>,
                )
                    .chain()
                    .run_if(is_layout_present::<T>)
                    .in_set(UILayoutResolveSysSet),
            )
            .on_schedule(
                OnPostUpdate,
                (
                    change_style_system::<T>,
                    sync_intrinsic_system::<T>,
                    sync_projection_system::<T>,
                    flush_hierarchy_errors_system::<T>,
                    update_layout_system::<T>,
                    update_presentation_system::<T>,
                )
                    .chain()
                    .run_if(is_layout_present::<T>)
                    .in_set(UILayoutSysSet),
            )
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
    roots: Query<Entity, (With<UILayoutRoot<T>>, Allow<Disabled>)>,
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
        Added<UINode>,
        Added<UITransform>,
        Changed<Children>,
        Changed<ChildOf>,
    )>,
);

type ChangedUIMeasure<T> = (ManagedUI<T>, Changed<UIMeasure>);
type ChangedBuiltinIntrinsic<T> = (
    ManagedUI<T>,
    Or<(Changed<UIText>, Changed<UIRichText>, Changed<UIImage>)>,
);

#[derive(QueryData)]
struct UIBuiltinIntrinsic {
    entity: Entity,
    owner: &'static UILayoutOwner,
    text: Option<Ref<'static, UIText>>,
    rich_text: Option<Ref<'static, UIRichText>>,
    image: Option<Ref<'static, UIImage>>,
}

#[derive(SystemParam)]
struct UIProjectionRemovals<'w, 's, T: Component> {
    marker: RemovedComponents<'w, 's, T>,
    owner: RemovedComponents<'w, 's, UILayoutOwner>,
    style: RemovedComponents<'w, 's, UIStyle>,
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
    branches: &Query<UIProjection<T>, Allow<Disabled>>,
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
    branches: Query<UIProjection<T>, Allow<Disabled>>,
    changed: Query<UIProjection<T>, ChangedUIProjection<T>>,
    owned: Query<(Entity, &UILayoutOwner), ManagedUI<T>>,
    roots: Query<
        (Option<Ref<Children>>, Option<Ref<ChildOf>>),
        (With<UILayoutRoot<T>>, Allow<Disabled>),
    >,
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

fn flush_hierarchy_errors_system<T: Component>(
    mut layout: ResMut<UILayout<T>>,
    mut commands: Commands,
) {
    for (entity, reason) in layout.take_hierarchy_errors() {
        commands.trigger(UIRuntimeError::InvalidHierarchy { entity, reason });
    }
}

fn update_layout_system<T: Component>(world: &mut World) {
    let updated =
        world.resource_scope(|world, mut layout: Mut<UILayout<T>>| layout.update_from_world(world));
    if updated {
        world.resource_scope(|world, layout: Mut<UILayout<T>>| {
            let mut nodes = world
                .query_filtered::<(Entity, &mut UINode, Option<&mut UIScroll>), ManagedUI<T>>();
            for (entity, mut node, scroll) in nodes.iter_mut(world) {
                if !layout.contains(entity) || !layout.set_node_layout(entity, &mut node) {
                    continue;
                }
                if let Some(mut scroll) = scroll
                    && let Some(height) = layout.scroll_height(entity)
                {
                    scroll.set_max_offset(height);
                }
            }
        });
        world.write_message(UILayoutUpdateEvent::<T>::default());
    }
    let errors = world.resource_mut::<UILayout<T>>().take_measure_errors();
    for (entity, size) in errors {
        world.trigger(UIRuntimeError::InvalidMeasure { entity, size });
    }
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

#[cfg(feature = "ecs-ui-experimental")]
pub(super) fn experimental_layout_root<T: Component>(world: &mut World) -> Option<Entity> {
    let root = world.get_resource::<UILayoutInstalled<T>>()?.root;
    let root = world.get_resource_mut::<UILayout<T>>()?.bind_root(root);
    valid_layout_root::<T>(world, root).then_some(root)
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
        .run_system_once(flush_hierarchy_errors_system::<T>)
        .or_panic("Reporting ECS UI hierarchy errors");
    world
        .run_system_once(update_layout_system::<T>)
        .or_panic("Updating ECS UI layout");
}

fn reset_pointer_lifecycle(
    transitions: &mut ResolvedPointerTransitions,
    pointer: &mut UIPointer,
    entity: Entity,
    position: Option<UIPointerPosition>,
) -> bool {
    if let Some(position) = position {
        pointer.set_position(position);
    }
    let just_exited = pointer.reset_lifecycle();
    if just_exited && let Some(position) = position {
        transitions.push(ResolvedPointerTransition::Leave(UIPointerLeave {
            entity,
            position,
        }));
    }
    just_exited
}

fn reset_unprojected_pointers_system<T: Component>(
    mut layout: ResMut<UILayout<T>>,
    mut transitions: ResMut<ResolvedPointerTransitions>,
    mut pointers: Query<(&mut UIPointer, Option<&UINode>), Allow<Disabled>>,
    mouse: Res<Mouse>,
) {
    let cursor = mouse.position();
    let mut unprojected = layout.take_unprojected();
    unprojected.retain(|&entity| {
        let Ok((mut pointer, node)) = pointers.get_mut(entity) else {
            return false;
        };
        let position = node
            .map(|node| resolved_pointer_position(&layout, cursor, node))
            .or_else(|| pointer.last_position());
        reset_pointer_lifecycle(&mut transitions, &mut pointer, entity, position)
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
            UINodeGraph::End => {
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
        (Entity, &UIStyle, Has<UIScroll>),
        (ManagedUI<T>, Or<(Changed<UIStyle>, Added<UIScroll>)>),
    >,
    nodes: Query<(Entity, &UIStyle), ManagedUI<T>>,
    mut removed_scrolls: RemovedComponents<UIScroll>,
    mut layout: ResMut<UILayout<T>>,
) {
    for (entity, style, scroll) in &changed {
        layout.set_node_style(entity, style, scroll);
    }
    for entity in removed_scrolls.read() {
        if let Ok((entity, style)) = nodes.get(entity) {
            layout.set_node_style(entity, style, false);
        }
    }
}

fn sync_intrinsic_system<T: Component>(
    changed_measures: Query<(Entity, &UILayoutOwner, &UIMeasure), ChangedUIMeasure<T>>,
    changed_builtins: Query<UIBuiltinIntrinsic, ChangedBuiltinIntrinsic<T>>,
    owners: Query<&UILayoutOwner, ManagedUI<T>>,
    mut removed_measures: RemovedComponents<UIMeasure>,
    mut removed_texts: RemovedComponents<UIText>,
    mut removed_rich_texts: RemovedComponents<UIRichText>,
    mut removed_images: RemovedComponents<UIImage>,
    mut layout: ResMut<UILayout<T>>,
) {
    let root = layout.root_entity();
    for builtin in &changed_builtins {
        if builtin.owner.root != root {
            continue;
        }
        for (source, tick) in [
            (
                UIIntrinsicSource::Text,
                builtin
                    .text
                    .as_ref()
                    .filter(|text| text.is_changed())
                    .map(|text| text.last_changed()),
            ),
            (
                UIIntrinsicSource::RichText,
                builtin
                    .rich_text
                    .as_ref()
                    .filter(|text| text.is_changed())
                    .map(|text| text.last_changed()),
            ),
            (
                UIIntrinsicSource::Image,
                builtin
                    .image
                    .as_ref()
                    .filter(|image| image.is_changed())
                    .map(|image| image.last_changed()),
            ),
        ] {
            if let Some(tick) = tick {
                layout.sync_intrinsic_source(builtin.entity, source, Some(tick));
            }
        }
    }

    let removed = removed_texts
        .read()
        .map(|entity| (entity, UIIntrinsicSource::Text))
        .chain(
            removed_rich_texts
                .read()
                .map(|entity| (entity, UIIntrinsicSource::RichText)),
        )
        .chain(
            removed_images
                .read()
                .map(|entity| (entity, UIIntrinsicSource::Image)),
        );
    for (entity, source) in removed {
        let source_present = changed_builtins
            .get(entity)
            .is_ok_and(|builtin| match source {
                UIIntrinsicSource::Text => builtin.text.is_some(),
                UIIntrinsicSource::RichText => builtin.rich_text.is_some(),
                UIIntrinsicSource::Image => builtin.image.is_some(),
            });
        if source_present {
            continue;
        }
        if owners.get(entity).is_ok_and(|owner| owner.root == root) {
            layout.sync_intrinsic_source(entity, source, None);
        }
    }

    for (entity, owner, measure) in &changed_measures {
        if owner.root == root {
            layout.sync_intrinsic(entity, Some(measure.revision()));
        }
    }
    for entity in removed_measures.read() {
        if changed_measures.get(entity).is_ok() {
            continue;
        }
        if owners.get(entity).is_ok_and(|owner| owner.root == root) {
            layout.sync_intrinsic(entity, None);
        }
    }
}

fn point_in_rect(point: Vec2, size: Vec2) -> bool {
    point.x >= 0.0 && point.y >= 0.0 && point.x < size.x && point.y < size.y
}

fn resolved_pointer_position(
    layout: &UILayout<impl Component>,
    screen: Vec2,
    node: &UINode,
) -> UIPointerPosition {
    let local = layout.screen_to_node(screen, node);
    let parent = node.local_transform().transform_point2(local);
    UIPointerPosition {
        screen,
        local,
        parent,
    }
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
    mut nodes: Query<
        (&UINode, &UIStyle, Has<UIScroll>, Option<&mut UIPointer>),
        (With<T>, Allow<Disabled>),
    >,
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
            UINodeGraph::End => eligible = stack.pop().unwrap_or(true),
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
    mut transitions: ResMut<ResolvedPointerTransitions>,
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
    for index in (0..layout.graph.len()).rev() {
        let UINodeGraph::Node(entity) = layout.graph[index] else {
            continue;
        };
        let Ok((_, pointer, node, style, scroll, _)) = query.get_mut(entity) else {
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
    for index in (0..layout.graph.len()).rev() {
        let UINodeGraph::Node(entity) = layout.graph[index] else {
            continue;
        };
        let Some(current_delta) = scrolling else {
            break;
        };
        let Ok((_, mut pointer, node, style, mut scroll, policy)) = query.get_mut(entity) else {
            continue;
        };
        let has_scroll = scroll.is_some();
        if !wheel_target(&layout, cursor, &pointer, node, style, has_scroll) {
            continue;
        }

        pointer.scrolling = Some(current_delta);
        let position = resolved_pointer_position(&layout, cursor, node);
        transitions.push(ResolvedPointerTransition::Scroll(UIScrollInput {
            entity,
            delta: current_delta,
            position,
        }));
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
    mut query: Query<
        (
            &mut UIPointer,
            &UINode,
            Option<&UIPointerConsumePolicy>,
            Has<Disabled>,
        ),
        With<T>,
    >,
    layout: Res<UILayout<T>>,
    mut transitions: ResMut<ResolvedPointerTransitions>,
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

    for index in (0..layout.graph.len()).rev() {
        let UINodeGraph::Node(entity) = layout.graph[index] else {
            continue;
        };
        let Ok((mut pointer, node, policy, disabled)) = query.get_mut(entity) else {
            continue;
        };
        let position = resolved_pointer_position(&layout, cursor, node);
        if disabled {
            reset_pointer_lifecycle(&mut transitions, &mut pointer, entity, Some(position));
            continue;
        }
        let policy = policy.unwrap_or(&default_policy);
        let target_eligible = pointer.ancestor_eligible
            && node.is_visible()
            && point_in_rect(position.local, node.size);
        let is_hover = target_eligible && !consumed_hover;
        let just_enter = !pointer.is_hover && is_hover;
        let just_exit = pointer.is_hover && !is_hover;

        if is_hover && policy.on_hover {
            consumed_hover = true;
        }
        pointer.set_position(position);
        pointer.is_hover = is_hover;
        pointer.just_enter = just_enter;
        pointer.just_exit = just_exit;
        if just_enter {
            transitions.push(ResolvedPointerTransition::Enter(UIPointerEnter {
                entity,
                position,
            }));
        }
        if just_exit {
            transitions.push(ResolvedPointerTransition::Leave(UIPointerLeave {
                entity,
                position,
            }));
        }
        pointer.dragging.clear();

        for btn in MouseButton::iter() {
            let init_click = pointer.init_click.contains_key(&btn);
            let drag_started = pointer.init_drag.contains_key(&btn);
            let is_down = mouse.is_down(btn);
            let released = released_for_lifecycle.contains(btn);

            if drag_started && (released || !is_down) {
                let drag = UIDragEvent::End(position.parent);
                pointer.init_drag.remove(&btn);
                pointer.dragging.insert(btn, drag).unwrap();
                transitions.push(ResolvedPointerTransition::Drag(UIDragInput {
                    entity,
                    button: btn,
                    event: drag,
                    position,
                }));
            } else if is_moving && !released {
                let can_start = init_click && is_down && !drag_started && is_hover;
                let can_move = drag_started && is_down;
                if can_start {
                    let start_pos = pointer.init_click.get(&btn).copied().unwrap();
                    let drag = UIDragEvent::Start(position.parent);
                    pointer
                        .init_drag
                        .insert(btn, (start_pos, position.parent))
                        .unwrap();
                    pointer.dragging.insert(btn, drag).unwrap();
                    transitions.push(ResolvedPointerTransition::Drag(UIDragInput {
                        entity,
                        button: btn,
                        event: drag,
                        position,
                    }));
                } else if can_move {
                    let (start_pos, previous_pos) = pointer.init_drag.get(&btn).copied().unwrap();
                    let drag = UIDragEvent::Move {
                        start_pos,
                        current_pos: position.parent,
                        delta: position.parent - previous_pos,
                    };
                    pointer.dragging.insert(btn, drag).unwrap();
                    pointer
                        .init_drag
                        .insert(btn, (start_pos, position.parent))
                        .unwrap();
                    transitions.push(ResolvedPointerTransition::Drag(UIDragInput {
                        entity,
                        button: btn,
                        event: drag,
                        position,
                    }));
                }
            }
            if !is_down && !released {
                pointer.init_click.remove(&btn);
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
                            pointer.init_click.insert(btn, position.parent).unwrap();
                            transitions.push(ResolvedPointerTransition::Pressed(
                                UIPointerPressed {
                                    entity,
                                    button: btn,
                                    position,
                                },
                            ));
                            if policy.on_pressed.contains(&btn) {
                                pressed_buttons.remove(btn);
                            }
                            if policy.block_global_pressed.contains(&btn) {
                                mouse.clear_pressed_btn(btn);
                            }
                        }
                        ButtonEdge::Released => {
                            pointer.released.insert(btn).unwrap();
                            transitions.push(ResolvedPointerTransition::Released(
                                UIPointerReleased {
                                    entity,
                                    button: btn,
                                    position,
                                },
                            ));
                            if policy.on_released.contains(&btn) {
                                released_buttons.remove(btn);
                            }
                            if policy.block_global_released.contains(&btn) {
                                mouse.clear_released_btn(btn);
                            }
                            if pointer.init_click.contains_key(&btn) && !consumed_click {
                                pointer.clicked.insert(btn).unwrap();
                                transitions.push(ResolvedPointerTransition::Click(UIClick {
                                    entity,
                                    button: btn,
                                    position,
                                }));
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
