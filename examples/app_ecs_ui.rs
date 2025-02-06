use draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::Vec2;
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

fn compute_layout_system(mut query: Query<&mut UINode>, mut layouts: ResMut<UILayouts>) {
    layouts.layouts.iter_mut().for_each(|(id, layout)| {
        let updated = layout.update();
        // TODO: update UINode info we need to use UIStyle as input and UIWhatever as output
        layout.tree.child_ids(parent_node_id)
    });
}

#[derive(Component)]
pub struct UINode {
    pub id: NodeId,
    pub layout: UILayoutId,
    pub position: Vec2,
    pub size: Vec2,
    // TODO: extras as content_size, etc...
}

#[derive(Debug)]
struct LayoutData {
    id: UILayoutId,
    dirty: bool,
    tree: TaffyTree<Entity>,
    size: Vec2,
    root: NodeId,
}

impl LayoutData {
    fn new(id: UILayoutId, size: Vec2) -> Self {
        let mut tree = TaffyTree::<Entity>::new();
        let root = tree.new_leaf(Style::default()).unwrap();
        Self {
            id,
            dirty: true,
            tree,
            root,
            size,
        }
    }

    pub fn set_size(&mut self, size: Vec2) {
        self.size = size;
        self.dirty = true;
    }

    pub fn add_node(&mut self, entity: Entity, style: Style, parent: Option<NodeId>) -> NodeId {
        self.dirty = true;
        let node_id = self.tree.new_leaf_with_context(style, entity).unwrap();
        if let Some(parent) = parent {
            self.tree.add_child(parent, node_id).unwrap();
        }
        node_id
    }

    pub fn update(&mut self) -> bool {
        if !self.dirty {
            return false;
        }

        self.dirty = false;
        self.tree
            .compute_layout(
                self.root,
                Size {
                    width: AvailableSpace::Definite(self.size.x),
                    height: AvailableSpace::Definite(self.size.y),
                },
            )
            .unwrap();

        return true;
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

#[derive(Component, Default)]
pub struct UIStyle {}

impl UIStyle {
    fn to_taffy(&self) -> Style {
        Style::default()
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
                });
            },
        ));
        self.stack.push(self.count);
        self
    }

    pub fn with_children<F: FnOnce(&mut Self)>(&mut self, cb: F) -> &mut Self {
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

fn setup_system(mut cmds: Commands, win: Res<Window>) {
    cmds.spawn_ui(
        UILayoutId("main"),
        (UIPos(Vec2::ZERO), UITint(Color::WHITE), UISize(win.size())),
    )
    .with_children(|cmd| {
        cmd.add(((
            UIPos(Vec2::splat(20.0)),
            UITint(Color::ORANGE),
            UISize(Vec2::splat(400.0)),
        ),))
            .with_children(|cmd| {
                cmd.add((
                    UIPos(Vec2::splat(20.0)),
                    UITint(Color::GREEN),
                    UISize(Vec2::splat(40.0)),
                ));
            });

        cmd.add((
            UIPos(Vec2::splat(500.0)),
            UITint(Color::BLUE),
            UISize(Vec2::splat(40.0)),
        ));
    });

    cmds.spawn_ui(
        UILayoutId("main"),
        (
            UIPos(Vec2::splat(20.0) + Vec2::splat(25.0)),
            UITint(Color::ORANGE),
            UISize(Vec2::splat(400.0)),
        ),
    )
    .with_children(|cmd| {
        cmd.add((
            UIPos(Vec2::splat(20.0) + Vec2::splat(25.0)),
            UITint(Color::GREEN),
            UISize(Vec2::splat(40.0)),
        ));

        cmd.add((
            UIPos(Vec2::splat(500.0) + Vec2::splat(25.0)),
            UITint(Color::BLUE),
            UISize(Vec2::splat(40.0)),
        ));
    });
}

fn draw_system(query: Query<(&UIPos, &UITint, &UISize)>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    for (pos, tint, size) in &query {
        draw.rect(Vec2::ZERO, size.0).translate(pos.0).color(tint.0);
    }

    gfx::render_to_frame(&draw).unwrap();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct UILayoutId(&'static str);
