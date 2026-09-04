use crate::{
    draw::{BaseCam2D, Camera2D, Draw2D},
    math::{Mat3, Mat4, Rect, Vec2, Vec3Swizzles, vec2, vec3},
};
use bevy_ecs::{change_detection::Tick, prelude::*, query::QueryData};
use corelib::math::{orthographic, vec4};
use rustc_hash::{FxHashMap, FxHashSet};
use std::ops::Range;
use taffy::prelude::*;

use super::{
    components::{UINode, UIRender, UIScroll, UITransform},
    ctx::{NodeContext, UINodeType, measure},
    style::{UIOverflow, UIStyle},
    widgets::{UIImage, UIRichText, UIText},
};

#[derive(Component)]
pub(super) struct UILayoutRoot<T: Component>(std::marker::PhantomData<T>);

impl<T: Component> UILayoutRoot<T> {
    pub(super) fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

#[derive(Component, Clone, Copy)]
pub(super) struct UILayoutOwner {
    pub(super) root: Entity,
}

pub(super) type ManagedUI<T> = (With<T>, With<UILayoutOwner>);

#[derive(Clone, Copy, Debug)]
pub(super) enum UINodeGraph {
    Node(Entity),
    Begin(Entity),
    End(Entity),
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(super) enum UIProjectionField {
    Marker,
    Owner,
    Style,
    NodeType,
    Node,
    Transform,
    Children,
    Parent,
}

impl UIProjectionField {
    const COUNT: usize = Self::Parent as usize + 1;
}

#[derive(Debug, Default)]
struct UIProjectionStamp {
    fields: [Option<Tick>; UIProjectionField::COUNT],
    intrinsic: Option<Tick>,
}

impl UIProjectionStamp {
    fn field(&mut self, field: UIProjectionField) -> &mut Option<Tick> {
        &mut self.fields[field as usize]
    }

    fn observe(&mut self, field: UIProjectionField, tick: Tick) -> bool {
        self.field(field).replace(tick) != Some(tick)
    }

    fn remove(&mut self, field: UIProjectionField) -> bool {
        self.field(field).take().is_some()
    }

    fn observe_intrinsic(&mut self, tick: Tick) -> bool {
        self.intrinsic.replace(tick) != Some(tick)
    }
}

#[derive(QueryData)]
pub(super) struct UIProjection<T: Component> {
    pub(super) entity: Entity,
    pub(super) marker: Option<Ref<'static, T>>,
    pub(super) owner: Option<Ref<'static, UILayoutOwner>>,
    pub(super) style: Option<Ref<'static, UIStyle>>,
    pub(super) typ: Option<Ref<'static, UINodeType>>,
    pub(super) scroll: Has<UIScroll>,
    pub(super) node: Option<Ref<'static, UINode>>,
    pub(super) transform: Option<Ref<'static, UITransform>>,
    pub(super) children: Option<Ref<'static, Children>>,
    pub(super) parent: Option<Ref<'static, ChildOf>>,
}

#[derive(Clone, Copy, Debug)]
enum UILayoutRootState {
    Unbound,
    Present(Entity),
    Missing(Entity),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum UIHierarchyProblem {
    MissingRoot,
    MissingMarker,
    MissingOwner,
    MissingStyle,
    MissingNodeType,
    MissingNode,
    MissingTransform,
    WrongOwner,
    WrongParent,
    Cycle,
}

impl<T: Component> UIProjectionItem<'_, '_, T> {
    fn hierarchy_problem(&self, root: Entity, parent: Entity) -> Option<UIHierarchyProblem> {
        if self.owner.as_ref().is_some_and(|owner| owner.root != root) {
            Some(UIHierarchyProblem::WrongOwner)
        } else if self.marker.is_none() {
            Some(UIHierarchyProblem::MissingMarker)
        } else if self.owner.is_none() {
            Some(UIHierarchyProblem::MissingOwner)
        } else if self.style.is_none() {
            Some(UIHierarchyProblem::MissingStyle)
        } else if self.typ.is_none() {
            Some(UIHierarchyProblem::MissingNodeType)
        } else if self.node.is_none() {
            Some(UIHierarchyProblem::MissingNode)
        } else if self.transform.is_none() {
            Some(UIHierarchyProblem::MissingTransform)
        } else if self
            .parent
            .as_ref()
            .is_none_or(|child_of| child_of.parent() != parent)
        {
            Some(UIHierarchyProblem::WrongParent)
        } else {
            None
        }
    }

    pub(super) fn component_tick(&self, field: UIProjectionField) -> Option<Tick> {
        self.field_tick(field, false)
    }

    pub(super) fn changed_component_tick(&self, field: UIProjectionField) -> Option<Tick> {
        self.field_tick(field, true)
    }

    fn field_tick(&self, field: UIProjectionField, changed_only: bool) -> Option<Tick> {
        match field {
            UIProjectionField::Marker => projection_tick(self.marker.as_ref(), changed_only),
            UIProjectionField::Owner => projection_tick(self.owner.as_ref(), changed_only),
            UIProjectionField::Style => projection_tick(self.style.as_ref(), changed_only),
            UIProjectionField::NodeType => projection_tick(self.typ.as_ref(), changed_only),
            UIProjectionField::Node => projection_tick(self.node.as_ref(), changed_only),
            UIProjectionField::Transform => projection_tick(self.transform.as_ref(), changed_only),
            UIProjectionField::Children => projection_tick(self.children.as_ref(), changed_only),
            UIProjectionField::Parent => projection_tick(self.parent.as_ref(), changed_only),
        }
    }
}

fn projection_tick<C: Component>(value: Option<&Ref<C>>, changed_only: bool) -> Option<Tick> {
    value
        .filter(|value| !changed_only || value.is_changed())
        .map(|value| value.last_changed())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct UICameraInfo {
    pub cam_size: Vec2,
    pub top_left: Vec2,
    pub layout_size: Vec2,
    pub projection: Mat4,
    pub inverse_projection: Mat4,
    pub transform: Mat3,
    pub inverse_transform: Mat3,
    pub pixel_perfect: bool,
}

impl UICameraInfo {
    fn from_base(cam: &Camera2D) -> Self {
        let bounds = cam.bounds();
        Self {
            cam_size: cam.size(),
            top_left: bounds.origin,
            layout_size: bounds.size,
            projection: cam.projection(),
            inverse_projection: cam.inverse_projection(),
            transform: cam.transform(),
            inverse_transform: cam.inverse_transform(),
            pixel_perfect: cam.is_pixel_perfect(),
        }
    }

    fn update_size(&mut self, size: Vec2) -> bool {
        if self.layout_size == size || size.x <= 0.0 || size.y <= 0.0 {
            return false;
        }
        self.cam_size = size;
        self.layout_size = size;
        self.projection = orthographic(0.0, size.x, size.y, 0.0, 0.0, 1.0);
        self.inverse_projection = self.projection.inverse();
        self.transform = Mat3::IDENTITY;
        self.inverse_transform = Mat3::IDENTITY;
        true
    }

    pub fn screen_to_local(&self, screen_pos: Vec2, local_inverse_transform: Mat3) -> Vec2 {
        let norm = screen_pos / self.cam_size;
        let mouse_pos = norm * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
        let pos = self
            .inverse_projection
            .project_point3(vec3(mouse_pos.x, mouse_pos.y, 1.0));
        local_inverse_transform.transform_point2(pos.xy())
    }
}

#[derive(Debug, Resource)]
pub struct UILayout<T: Component> {
    _m: std::marker::PhantomData<T>,
    root_state: UILayoutRootState,
    root: NodeId,
    tree: TaffyTree<NodeContext>,
    relations: FxHashMap<Entity, NodeId>,
    graph_ranges: FxHashMap<Entity, Range<usize>>,
    diagnosed: FxHashSet<(Entity, UIHierarchyProblem)>,
    previous_diagnostics: FxHashSet<(Entity, UIHierarchyProblem)>,
    projection_stamps: FxHashMap<Entity, UIProjectionStamp>,
    unprojected: Vec<Entity>,
    reached: FxHashSet<Entity>,
    draw_ancestors: Vec<Entity>,
    dirty_layout: bool,
    dirty_topology: bool,
    pub(super) base_transform: Mat3,
    pub(super) graph: Vec<UINodeGraph>,
    pub(super) cam_info: UICameraInfo,
}

impl<T: Component> Default for UILayout<T> {
    fn default() -> Self {
        Self::new(UILayoutRootState::Unbound)
    }
}

impl<T: Component> UILayout<T> {
    pub(super) fn with_root(root: Entity) -> Self {
        Self::new(UILayoutRootState::Present(root))
    }

    fn new(root_state: UILayoutRootState) -> Self {
        let mut tree = TaffyTree::new();
        let root = tree.new_leaf(Style::default()).unwrap();
        let mut layout = Self {
            _m: std::marker::PhantomData,
            root_state,
            root,
            tree,
            relations: FxHashMap::default(),
            graph_ranges: FxHashMap::default(),
            diagnosed: FxHashSet::default(),
            previous_diagnostics: FxHashSet::default(),
            projection_stamps: FxHashMap::default(),
            unprojected: Vec::new(),
            reached: FxHashSet::default(),
            draw_ancestors: Vec::new(),
            dirty_layout: true,
            dirty_topology: true,
            base_transform: Mat3::IDENTITY,
            graph: Vec::new(),
            cam_info: UICameraInfo {
                cam_size: Vec2::ZERO,
                top_left: Vec2::ZERO,
                layout_size: Vec2::ZERO,
                projection: Mat4::IDENTITY,
                inverse_projection: Mat4::IDENTITY,
                transform: Mat3::IDENTITY,
                inverse_transform: Mat3::IDENTITY,
                pixel_perfect: false,
            },
        };
        layout.update_root();
        layout
    }

    fn update_root(&mut self) {
        self.tree
            .set_style(
                self.root,
                Style {
                    size: Size {
                        width: Dimension::Length(self.cam_info.layout_size.x),
                        height: Dimension::Length(self.cam_info.layout_size.y),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        self.base_transform = Mat3::from_translation(self.cam_info.top_left);
    }

    pub(super) fn root_entity(&self) -> Entity {
        match self.root_state {
            UILayoutRootState::Unbound => panic!("ECS UI layout root is not bound"),
            UILayoutRootState::Present(root) | UILayoutRootState::Missing(root) => root,
        }
    }

    pub(super) fn bind_root(&mut self, installed_root: Entity) -> Entity {
        match self.root_state {
            UILayoutRootState::Unbound => {
                self.root_state = UILayoutRootState::Present(installed_root);
                self.mark_topology_dirty();
                installed_root
            }
            UILayoutRootState::Present(root) | UILayoutRootState::Missing(root) => {
                debug_assert_eq!(root, installed_root);
                root
            }
        }
    }

    pub(super) fn report_missing_root(&mut self) {
        self.observe_root(false);
        self.report(self.root_entity(), UIHierarchyProblem::MissingRoot);
    }

    pub(super) fn mark_topology_dirty(&mut self) {
        self.dirty_topology = true;
    }

    pub(super) fn mark_layout_dirty(&mut self) {
        self.dirty_layout = true;
    }

    pub(super) fn observe_projection_change(
        &mut self,
        entity: Entity,
        component: UIProjectionField,
        tick: Tick,
    ) -> bool {
        self.projection_stamps
            .entry(entity)
            .or_default()
            .observe(component, tick)
    }

    pub(super) fn observe_projection_removal(
        &mut self,
        entity: Entity,
        component: UIProjectionField,
    ) -> bool {
        self.projection_stamps
            .get_mut(&entity)
            .is_some_and(|stamp| stamp.remove(component))
    }

    fn mark_projection_present(
        &mut self,
        entity: Entity,
        component: UIProjectionField,
        tick: Tick,
    ) {
        self.projection_stamps
            .entry(entity)
            .or_default()
            .observe(component, tick);
    }

    pub(super) fn observe_root(&mut self, present: bool) {
        let next = match (self.root_state, present) {
            (UILayoutRootState::Present(root), false) => UILayoutRootState::Missing(root),
            (UILayoutRootState::Missing(root), true) => UILayoutRootState::Present(root),
            _ => return,
        };
        self.root_state = next;
        self.mark_topology_dirty();
    }

    pub(super) fn forget_projection(&mut self, entity: Entity) {
        self.projection_stamps.remove(&entity);
    }

    pub(super) fn take_unprojected(&mut self) -> Vec<Entity> {
        std::mem::take(&mut self.unprojected)
    }

    pub(super) fn restore_unprojected(&mut self, entities: Vec<Entity>) {
        self.unprojected = entities;
    }

    /// Returns this node's parent in the projected UI hierarchy.
    #[inline]
    pub fn parent(&self, entity: Entity) -> Option<Entity> {
        self.relations
            .get(&entity)
            .and_then(|id| self.tree.parent(*id))
            .and_then(|parent| self.tree.get_node_context(parent))
            .map(|parent| parent.entity)
    }

    /// Returns the number of immediate children in the projected UI hierarchy.
    #[inline]
    pub fn child_count(&self, entity: Entity) -> usize {
        self.relations
            .get(&entity)
            .map_or(0, |node| self.tree.child_ids(*node).count())
    }

    /// Returns whether this node has children in the projected UI hierarchy.
    #[inline]
    pub fn has_children(&self, entity: Entity) -> bool {
        self.relations
            .get(&entity)
            .is_some_and(|node| self.tree.child_ids(*node).next().is_some())
    }

    /// Returns whether this node has no children in the projected UI hierarchy.
    #[inline]
    pub fn is_empty(&self, entity: Entity) -> bool {
        !self.has_children(entity)
    }

    /// Iterates this node's immediate children in projected hierarchy order.
    #[inline]
    pub fn children(&self, parent: Entity) -> impl Iterator<Item = Entity> + '_ {
        self.relations
            .get(&parent)
            .into_iter()
            .flat_map(move |node| self.tree.child_ids(*node))
            .filter_map(move |node| self.tree.get_node_context(node).map(|ctx| ctx.entity))
    }

    /// Converts screen coordinates to coordinates local to `node`.
    #[inline]
    pub fn screen_to_node(&self, screen_pos: Vec2, node: &UINode) -> Vec2 {
        self.cam_info.screen_to_local(
            screen_pos,
            node.global_transform().inverse() * self.cam_info.inverse_transform,
        )
    }

    /// Converts coordinates local to `node` into screen coordinates.
    #[inline]
    pub fn node_to_screen(&self, point: Vec2, node: &UINode) -> Vec2 {
        let transform = self.cam_info.transform * node.global_transform();
        let half = self.cam_info.cam_size * 0.5;
        let point = transform * vec3(point.x, point.y, 1.0);
        let point = self.cam_info.projection * vec4(point.x, point.y, point.z, 1.0);
        half + (half * vec2(point.x, -point.y))
    }

    /// Converts coordinates from `from`'s local space to `to`'s local space.
    #[inline]
    pub fn node_to_node(&self, point: Vec2, from: &UINode, to: &UINode) -> Vec2 {
        self.screen_to_node(self.node_to_screen(point, from), to)
    }

    /// Returns the current layout size.
    #[inline]
    pub fn size(&self) -> Vec2 {
        self.cam_info.layout_size
    }

    /// Sets the layout size and schedules a layout recomputation when it changes.
    #[inline]
    pub fn set_size(&mut self, size: Vec2) {
        if self.cam_info.update_size(size) {
            self.mark_layout_dirty();
            self.update_root();
        }
    }

    /// Updates layout camera data and schedules a recomputation when it changes.
    #[inline]
    pub fn set_camera(&mut self, cam: &Camera2D) {
        let size = cam.size();
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }
        let info = UICameraInfo::from_base(cam);
        if info != self.cam_info {
            self.cam_info = info;
            self.mark_layout_dirty();
            self.update_root();
        }
    }

    /// Recomputes dirty layout state and returns whether a computation occurred.
    pub fn update(
        &mut self,
        images: Query<&UIImage, With<T>>,
        rich_texts: Query<&UIRichText, With<T>>,
        texts: Query<&UIText, With<T>>,
    ) -> bool {
        if !matches!(self.root_state, UILayoutRootState::Present(_)) || !self.dirty_layout {
            return false;
        }
        self.tree
            .compute_layout_with_measure(
                self.root,
                Size {
                    width: AvailableSpace::Definite(self.cam_info.layout_size.x),
                    height: AvailableSpace::Definite(self.cam_info.layout_size.y),
                },
                |known_dimensions, available_space, _node_id, ctx, _style| {
                    measure(
                        known_dimensions,
                        available_space,
                        ctx,
                        &images,
                        &rich_texts,
                        &texts,
                    )
                },
            )
            .unwrap();
        self.dirty_layout = false;
        true
    }

    pub(super) fn project_if_dirty(&mut self, branches: &Query<UIProjection<T>>) -> bool {
        if !self.dirty_topology {
            return false;
        }
        self.project(branches);
        matches!(self.root_state, UILayoutRootState::Present(_))
    }

    pub(super) fn report_unreached(
        &mut self,
        owned: &Query<(Entity, &UILayoutOwner), ManagedUI<T>>,
    ) {
        let root = self.root_entity();
        for (entity, owner) in owned {
            if owner.root == root && !self.reached.contains(&entity) {
                self.report(entity, UIHierarchyProblem::WrongParent);
                self.reached.insert(entity);
            }
        }
    }

    fn project(&mut self, branches: &Query<UIProjection<T>>) {
        self.dirty_topology = false;
        self.dirty_layout = true;
        self.previous_diagnostics.clear();
        std::mem::swap(&mut self.diagnosed, &mut self.previous_diagnostics);
        self.unprojected.extend(self.relations.keys().copied());
        self.relations.clear();
        self.graph.clear();
        self.graph_ranges.clear();
        self.tree.clear();
        self.root = self.tree.new_leaf(Style::default()).unwrap();
        self.update_root();

        let root = self.root_entity();
        if matches!(self.root_state, UILayoutRootState::Missing(_)) {
            self.projection_stamps.retain(|entity, _| *entity == root);
            self.report(root, UIHierarchyProblem::MissingRoot);
            return;
        }
        let mut reached = std::mem::take(&mut self.reached);
        reached.clear();
        reached.insert(root);
        self.project_children(branches, root, root, self.root, &mut reached);
        self.unprojected
            .retain(|entity| !self.relations.contains_key(entity));
        self.reached = reached;
    }

    fn project_children(
        &mut self,
        branches: &Query<UIProjection<T>>,
        root: Entity,
        parent_entity: Entity,
        parent_node: NodeId,
        reached: &mut FxHashSet<Entity>,
    ) {
        let Ok(parent) = branches.get(parent_entity) else {
            return;
        };
        let Some(children) = parent.children else {
            return;
        };
        for child in children.iter() {
            if !reached.insert(child) {
                self.report(child, UIHierarchyProblem::Cycle);
                continue;
            }
            let Ok(branch) = branches.get(child) else {
                self.report(child, UIHierarchyProblem::MissingMarker);
                continue;
            };
            let problem = branch.hierarchy_problem(root, parent_entity);
            if let Some(problem) = problem {
                self.report(child, problem);
                continue;
            }
            for component in [
                UIProjectionField::Marker,
                UIProjectionField::Owner,
                UIProjectionField::Style,
                UIProjectionField::NodeType,
                UIProjectionField::Node,
                UIProjectionField::Transform,
                UIProjectionField::Children,
                UIProjectionField::Parent,
            ] {
                if let Some(tick) = branch.component_tick(component) {
                    self.mark_projection_present(child, component, tick);
                }
            }
            let node_id = self
                .tree
                .new_leaf_with_context(
                    branch.style.unwrap().as_taffy_style(branch.scroll),
                    NodeContext {
                        entity: child,
                        typ: *branch.typ.unwrap(),
                    },
                )
                .unwrap();
            self.tree.add_child(parent_node, node_id).unwrap();
            self.relations.insert(child, node_id);
            let start = self.graph.len();
            self.graph.push(UINodeGraph::Begin(child));
            self.graph.push(UINodeGraph::Node(child));
            self.project_children(branches, root, child, node_id, reached);
            self.graph.push(UINodeGraph::End(child));
            self.graph_ranges.insert(child, start..self.graph.len());
        }
    }

    fn report(&mut self, entity: Entity, problem: UIHierarchyProblem) {
        let diagnostic = (entity, problem);
        if self.diagnosed.insert(diagnostic) && !self.previous_diagnostics.contains(&diagnostic) {
            log::warn!("Ignoring invalid ECS UI hierarchy at {entity:?}: {problem:?}");
        }
    }

    pub(super) fn node_id(&self, entity: Entity) -> Option<NodeId> {
        self.relations.get(&entity).copied()
    }

    pub(super) fn contains(&self, entity: Entity) -> bool {
        self.relations.contains_key(&entity)
    }

    pub(super) fn set_node_style(&mut self, entity: Entity, style: &UIStyle, scroll: bool) {
        let Some(node) = self.node_id(entity) else {
            return;
        };
        let style = style.as_taffy_style(scroll);
        if self.tree.style(node).unwrap() != &style {
            self.tree.set_style(node, style).unwrap();
            self.mark_layout_dirty();
        }
    }

    pub(super) fn invalidate_intrinsic(&mut self, entity: Entity, tick: Tick) {
        let changed = self
            .projection_stamps
            .entry(entity)
            .or_default()
            .observe_intrinsic(tick);
        if changed && let Some(node) = self.node_id(entity) {
            self.tree.mark_dirty(node).unwrap();
            self.mark_layout_dirty();
        }
    }

    pub(super) fn scroll_height(&self, entity: Entity) -> Option<f32> {
        self.node_id(entity)
            .and_then(|node| self.tree.layout(node).ok())
            .map(|layout| layout.scroll_height())
    }

    pub(super) fn set_node_layout(&self, entity: Entity, node: &mut UINode) -> bool {
        let Some(node_id) = self.node_id(entity) else {
            return false;
        };
        let layout = self.tree.layout(node_id).unwrap();
        let size = vec2(layout.size.width, layout.size.height);
        let position = vec2(layout.location.x, layout.location.y);
        if self.cam_info.pixel_perfect {
            node.size = size.round();
            node.position = position.round();
        } else {
            node.size = size;
            node.position = position;
        }
        true
    }

    fn graph_slice(&self, from: Option<Entity>) -> Option<&[UINodeGraph]> {
        from.map_or(Some(self.graph.as_slice()), |entity| {
            self.graph_ranges
                .get(&entity)
                .map(|range| &self.graph[range.clone()])
        })
    }
}

fn clip_state(world: &World, entity: Entity) -> Option<(Mat3, Vec2, UIOverflow)> {
    let node = world.get::<UINode>(entity)?;
    if !node.is_visible() {
        return None;
    }
    let style = world.get::<UIStyle>(entity)?;
    let overflow = style.effective_overflow(world.get::<UIScroll>(entity).is_some());
    (!matches!(overflow, UIOverflow::Visible)).then_some((
        node.global_transform,
        node.size(),
        overflow,
    ))
}

fn push_clip(draw: &mut Draw2D, transform: Mat3, size: Vec2, overflow: UIOverflow) {
    draw.push_matrix(transform);
    let rect = Rect::new(Vec2::ZERO, size);
    match overflow {
        UIOverflow::Visible => {}
        UIOverflow::Clip => draw.push_clip(rect),
        UIOverflow::Rounded(radius) => draw.push_rounded_clip(rect, radius),
    }
    draw.pop_matrix();
}

pub fn draw_ui_layout<T: Component>(draw: &mut Draw2D, world: &mut World) {
    draw_ui_layout_from::<T>(draw, world, None)
}

pub fn draw_ui_layout_from<T: Component>(
    draw: &mut Draw2D,
    world: &mut World,
    from: Option<Entity>,
) {
    world.resource_scope(|world: &mut World, mut layout: Mut<UILayout<T>>| {
        let mut ancestors = std::mem::take(&mut layout.draw_ancestors);
        ancestors.clear();
        if let Some(from) = from {
            let mut parent = layout.parent(from);
            while let Some(entity) = parent {
                ancestors.push(entity);
                parent = layout.parent(entity);
            }
            ancestors.reverse();
        }
        let Some(graph) = layout.graph_slice(from) else {
            layout.draw_ancestors = ancestors;
            return;
        };
        let mut seeded_clips = 0;
        for &entity in &ancestors {
            if let Some((transform, size, overflow)) = clip_state(world, entity) {
                push_clip(draw, transform, size, overflow);
                seeded_clips += 1;
            }
        }
        for event in graph {
            match event {
                UINodeGraph::Begin(_) => {}
                UINodeGraph::Node(entity) => {
                    let Some(node) = world.get::<UINode>(*entity).copied() else {
                        continue;
                    };
                    let alpha = draw.alpha();
                    draw.set_alpha(alpha * node.global_alpha);
                    draw.push_matrix(node.global_transform);
                    if node.is_visible()
                        && let Some(render) = world.get::<UIRender>(*entity)
                    {
                        render.render(draw, world, *entity);
                    }
                    if let Some((_, size, overflow)) = clip_state(world, *entity) {
                        let rect = Rect::new(Vec2::ZERO, size);
                        match overflow {
                            UIOverflow::Visible => {}
                            UIOverflow::Clip => draw.push_clip(rect),
                            UIOverflow::Rounded(radius) => draw.push_rounded_clip(rect, radius),
                        }
                    }
                    draw.pop_matrix();
                    draw.set_alpha(alpha);
                }
                UINodeGraph::End(entity) => {
                    if clip_state(world, *entity).is_some() {
                        draw.pop_clip();
                    }
                }
            }
        }
        for _ in 0..seeded_clips {
            draw.pop_clip();
        }
        layout.draw_ancestors = ancestors;
    });
}
