use draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::{vec2, Mat3, Vec2};
use rkit::nui::style::{Style as NStyle, Unit};
use rkit::prelude::*;
use rustc_hash::FxHashMap;

use taffy::prelude::*;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(AddMainPlugins::default())
        .add_plugin(UIPlugin)
        .add_systems(OnSetup, setup_system)
        .add_systems(OnRender, draw_system)
        .run()
}

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn apply(self, app: App) -> App {
        app.add_resource(UILayouts::default())
            .add_systems(OnPostUpdate, compute_layout_system)
    }
}

fn compute_layout_system(
    mut query: Query<&mut UINode>,
    mut layouts: ResMut<UILayouts>,
    win: Res<Window>,
) {
    layouts.layouts.iter_mut().for_each(|(id, layout)| {
        layout.set_size(win.size()); // TODO: fixme
        let updated = layout.update();
        if !updated {
            return;
        }

        query
            .iter_mut()
            .filter(|ui_node| ui_node.layout == layout.id)
            .for_each(|mut ui_node| {
                let l = layout.tree.layout(ui_node.id).unwrap();
                ui_node.size = vec2(l.size.width, l.size.height);
                ui_node.position = vec2(l.location.x, l.location.y);
            });
    });
}

#[derive(Component, Debug)]
pub struct UINode {
    pub id: NodeId,
    pub layout: UILayoutId,
    pub position: Vec2,
    pub size: Vec2,
    // TODO: extras as content_size, etc...
}

#[derive(Clone, Copy, Debug)]
enum UINodeGraph {
    Node(Entity),
    Begin(Entity),
    End(Entity),
}

#[derive(Debug)]
struct LayoutData {
    id: UILayoutId,
    dirty_layout: bool,
    dirty_graph: bool,
    tree: TaffyTree<Entity>,
    graph: Vec<UINodeGraph>,
    size: Vec2,
    root: NodeId,
}

impl LayoutData {
    fn new(id: UILayoutId, size: Vec2) -> Self {
        let mut tree = TaffyTree::<Entity>::new();
        let root = tree.new_leaf(Style::default()).unwrap();
        Self {
            id,
            dirty_layout: true,
            dirty_graph: true,
            tree,
            graph: vec![],
            root,
            size,
        }
    }

    pub fn set_size(&mut self, size: Vec2) {
        if size == self.size {
            return;
        }

        self.size = size;
        self.tree
            .set_style(
                self.root,
                Style {
                    size: Size {
                        width: Dimension::Length(size.x),
                        height: Dimension::Length(size.y),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        self.dirty_layout = true;
    }

    pub fn add_node(&mut self, entity: Entity, style: Style, parent: Option<NodeId>) -> NodeId {
        self.dirty_layout = true;
        self.dirty_graph = true;
        let node_id = self.tree.new_leaf_with_context(style, entity).unwrap();
        let parent_id = parent.unwrap_or(self.root);
        self.tree.add_child(parent_id, node_id).unwrap();
        node_id
    }

    pub fn update(&mut self) -> bool {
        let needs_compute = self.dirty_graph || self.dirty_layout;
        if needs_compute {
            self.tree
                .compute_layout(
                    self.root,
                    Size {
                        width: AvailableSpace::Definite(self.size.x),
                        height: AvailableSpace::Definite(self.size.y),
                    },
                )
                .unwrap();
        }

        if self.dirty_graph {
            self.graph.clear();
            process_graph(&mut self.graph, self.root, &self.tree);
        }

        self.dirty_layout = false;
        self.dirty_graph = false;
        needs_compute
    }
}

fn process_graph(graph: &mut Vec<UINodeGraph>, node_id: NodeId, tree: &TaffyTree<Entity>) {
    match tree.get_node_context(node_id).cloned() {
        Some(e) => {
            graph.push(UINodeGraph::Begin(e));
            graph.push(UINodeGraph::Node(e));
            tree.child_ids(node_id)
                .for_each(|child_id| process_graph(graph, child_id, tree));
            graph.push(UINodeGraph::End(e));
        }
        _ => tree
            .child_ids(node_id)
            .for_each(|child_id| process_graph(graph, child_id, tree)),
    }
}

#[derive(Default, Debug, Resource)]
pub struct UILayouts {
    layouts: FxHashMap<UILayoutId, LayoutData>,
}

impl UILayouts {
    fn add(
        &mut self,
        entity: Entity,
        layout_id: UILayoutId,
        style: Style,
        parent: Option<NodeId>,
    ) -> NodeId {
        let layout = self
            .layouts
            .entry(layout_id)
            .or_insert_with(|| LayoutData::new(layout_id, Vec2::splat(10.0)));

        layout.add_node(entity, style, parent)
    }

    pub fn graph(&self, layout: UILayoutId) -> Option<&[UINodeGraph]> {
        self.layouts
            .get(&layout)
            .map(|layout| layout.graph.as_slice())
    }
}

pub struct SpawnUICommandBuilder<'c, 'w, 's> {
    cmds: Option<&'c mut Commands<'w, 's>>,
    count: usize,
    stack: Vec<usize>,
    layout: UILayoutId,
    bundles: Option<Vec<Box<dyn FnOnce(&mut World, &mut FxHashMap<usize, NodeId>) + Send>>>,
}

pub struct SpawnUICommand {
    bundles: Vec<Box<dyn FnOnce(&mut World, &mut FxHashMap<usize, NodeId>) + Send>>,
}

#[derive(Component, Default, Deref)]
pub struct UIStyle(NStyle);

impl UIStyle {
    fn to_taffy(&self) -> Style {
        self.0.to_taffy()
    }
}

impl SpawnUICommandBuilder<'_, '_, '_> {
    pub fn add<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        self.count += 1;
        let id = self.count;
        let parent_id = self.stack.last().cloned();
        let layout_id = self.layout;
        self.bundles.as_mut().unwrap().push(Box::new(
            move |world: &mut World, ids: &mut FxHashMap<usize, NodeId>| {
                let (entity, style) = world
                    .spawn(UIStyle::default())
                    .insert(bundle)
                    .get_components::<(Entity, &UIStyle)>()
                    .map(|(e, style)| (e, style.to_taffy()))
                    .unwrap();

                let mut layouts = world.get_resource_mut::<UILayouts>().unwrap();
                let parent_id = parent_id.and_then(|p_id| ids.get(&p_id)).cloned();
                let node_id = layouts.add(entity, layout_id, style, parent_id);
                ids.insert(id, node_id);

                world.entity_mut(entity).insert(UINode {
                    layout: layout_id,
                    id: node_id,
                    position: Vec2::ZERO,
                    size: Vec2::ONE,
                });
            },
        ));
        self
    }

    pub fn with_children<F: FnOnce(&mut Self)>(&mut self, cb: F) -> &mut Self {
        self.stack.push(self.count);
        cb(self);
        self.stack.pop();
        self
    }
}

impl Drop for SpawnUICommandBuilder<'_, '_, '_> {
    fn drop(&mut self) {
        let bundles = self.bundles.take();
        let command = SpawnUICommand {
            bundles: bundles.unwrap(),
        };
        let cmds = self.cmds.take().unwrap();
        cmds.queue(command);
    }
}

pub trait CommandSpawnUIExt<'w, 's> {
    fn spawn_ui<'c, T: Bundle>(
        &'c mut self,
        layout: UILayoutId,
        bundle: T,
    ) -> SpawnUICommandBuilder<'c, 'w, 's>;
}

impl<'w, 's> CommandSpawnUIExt<'w, 's> for Commands<'w, 's> {
    fn spawn_ui<'c, T: Bundle>(
        &'c mut self,
        layout: UILayoutId,
        bundle: T,
    ) -> SpawnUICommandBuilder<'c, 'w, 's> {
        let mut builder = SpawnUICommandBuilder {
            cmds: Some(self),
            count: 0,
            stack: vec![],
            bundles: Some(vec![]),
            layout,
        };

        builder.add(bundle);
        builder
    }
}

impl Command for SpawnUICommand {
    fn apply(self, world: &mut World) {
        let Self { bundles } = self;

        let mut table = FxHashMap::default();
        for cb in bundles {
            cb(world, &mut table);
        }
    }
}

#[derive(Component, Deref)]
pub struct UIPos(Vec2);

#[derive(Component, Deref)]
pub struct UITint(Color);

#[derive(Component, Deref)]
pub struct UISize(Vec2);

#[derive(Component, Deref)]
pub struct UIOrder(f32);

fn setup_system(mut cmds: Commands, win: Res<Window>) {
    cmds.spawn_ui(
        UILayoutId("main"),
        (
            UIStyle(
                NStyle::default()
                    .flex_row()
                    .size_full()
                    .align_items_center()
                    .justify_content_center(),
            ),
            UITint(Color::WHITE),
            UIOrder(0.0),
        ),
    )
    .with_children(|cmd| {
        cmd.add(((
            UIStyle(
                NStyle::default()
                    .flex_row()
                    .justify_content_space_evenly()
                    .size(Unit::Relative(0.9), Unit::Relative(0.9)),
            ),
            UITint(Color::ORANGE),
            UIOrder(1.0),
        ),))
            .with_children(|cmd| {
                cmd.add((
                    UIStyle(NStyle::default().size_auto().width(200.0)),
                    UITint(Color::RED),
                    UIOrder(2.0),
                ));
                cmd.add((
                    UIStyle(NStyle::default().size_auto().width(200.0)),
                    UITint(Color::GREEN),
                    UIOrder(3.0),
                ));
                cmd.add((
                    UIStyle(NStyle::default().size_auto().width(200.0)),
                    UITint(Color::BLUE),
                    UIOrder(4.0),
                ));
            });
    });
}

fn draw_system(query: Query<(&UINode, &UITint, &UIOrder)>, layouts: Res<UILayouts>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    if let Some(graph) = layouts.graph(UILayoutId("main")) {
        graph.iter().for_each(|ng| match ng {
            UINodeGraph::Begin(entity) => {
                let (node, _, _) = query.get(*entity).unwrap();
                draw.push_matrix(Mat3::from_translation(node.position));
            }
            UINodeGraph::Node(entity) => {
                let (node, tint, _) = query.get(*entity).unwrap();
                draw.rect(Vec2::ZERO, node.size).color(tint.0);
            }
            UINodeGraph::End(entity) => {
                draw.pop_matrix();
            }
        });
    }

    gfx::render_to_frame(&draw).unwrap();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct UILayoutId(&'static str);
