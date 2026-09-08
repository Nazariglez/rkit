use rkit::{
    draw::create_draw_2d,
    ecs::ui::{CommandUISceneExt, UIScene, ui},
    gfx::{self, Color},
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component)]
struct Workspace;

#[derive(Component)]
struct HierarchyStatus;

#[derive(Component, Clone, Copy)]
enum Parent {
    Left,
    Right,
}

impl Parent {
    fn label(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Component)]
struct Branch;

#[derive(Component)]
struct BranchLeaf;

#[derive(Component, Clone, Copy)]
struct Appended(usize);

#[derive(Component)]
struct NodeLabel(&'static str);

#[derive(Resource)]
struct HierarchyState {
    left: Entity,
    right: Entity,
    branch: Option<Entity>,
    next: usize,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .on_setup(setup)
        .on_pre_update(update_layout.before(UILayoutSysSet))
        .on_update((controls, show_hierarchy).chain())
        .on_render(draw)
        .run()
}

fn setup(mut commands: Commands) {
    let root = commands.spawn_ui(MainLayout, workspace());
    let row = commands.spawn_ui_children(MainLayout, root, [parent_row()])[0];
    let parents = commands.spawn_ui_children(
        MainLayout,
        row,
        [parent_panel(Parent::Left), parent_panel(Parent::Right)],
    );
    let left = parents[0];
    let right = parents[1];
    let branch = commands.spawn_ui_children(MainLayout, left, [branch()])[0];

    commands.insert_resource(HierarchyState {
        left,
        right,
        branch: Some(branch),
        next: 1,
    });
}

fn workspace() -> UIScene {
    ui::column()
        .insert(Workspace)
        .style(|style| style.size_full().align_items_center().padding(14.0))
        .style(|style| style.gap(8.0))
        .children([
            ui::text("Hierarchy mutation").style(|style| style.size(700.0, 28.0)),
            ui::text("1 Append | 2 Move | 3 Make root\n4 Despawn | 5 Reset layout cache")
                .style(|style| style.size(700.0, 44.0)),
            ui::text("")
                .insert(HierarchyStatus)
                .style(|style| style.size(700.0, 60.0)),
        ])
}

fn parent_row() -> UIScene {
    ui::row()
        .style(|style| style.size(700.0, 240.0).gap(20.0))
        .style(|style| style.align_items_start())
}

fn parent_panel(parent: Parent) -> UIScene {
    ui::container(UIContainer {
        bg_color: Some(Color::rgb(0.12, 0.18, 0.3)),
        border_color: Some(Color::WHITE),
        border_size: 2.0,
        corner_radius: Some(10.0),
    })
    .insert((parent, NodeLabel(parent.label())))
    .style(|style| style.size(340.0, 240.0).flex_col().padding(10.0))
    .style(|style| style.gap(8.0))
    .child(
        ui::text(format!("{} parent", parent.label()))
            .insert(NodeLabel("heading"))
            .style(|style| style.size(300.0, 24.0)),
    )
}

fn branch() -> UIScene {
    ui::container(UIContainer {
        bg_color: Some(Color::ORANGE),
        corner_radius: Some(8.0),
        ..Default::default()
    })
    .insert((Branch, NodeLabel("branch")))
    .style(|_| branch_style())
    .children([
        ui::text("stable branch entity").insert(NodeLabel("branch title")),
        ui::container(UIContainer {
            bg_color: Some(Color::RED),
            ..Default::default()
        })
        .insert((BranchLeaf, NodeLabel("recursive child")))
        .style(|style| style.size(280.0, 24.0))
        .child(ui::text("recursive child")),
    ])
}

fn branch_style() -> UIStyle {
    UIStyle::default()
        .size(300.0, 76.0)
        .flex_col()
        .padding(8.0)
        .gap(4.0)
}

fn appended(number: usize) -> UIScene {
    ui::container(UIContainer {
        bg_color: Some(Color::rgb(0.22, 0.48, 0.72)),
        ..Default::default()
    })
    .insert((Appended(number), NodeLabel("append")))
    .style(|style| style.size(300.0, 28.0).padding_x(6.0))
    .child(ui::text(format!("append {number}")))
}

fn update_layout(mut layout: ResMut<UILayout<MainLayout>>, window: Res<Window>) {
    layout.set_size(window.size());
}

fn controls(
    keyboard: Res<Keyboard>,
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    mut branch_styles: Query<&mut UIStyle, With<Branch>>,
    mut state: ResMut<HierarchyState>,
    mut layout: ResMut<UILayout<MainLayout>>,
) {
    if keyboard.just_pressed(KeyCode::Digit5) {
        let size = layout.size();
        *layout = UILayout::default();
        layout.set_size(size);
    }

    if keyboard.just_pressed(KeyCode::Digit1) {
        let first = state.next;
        commands.spawn_ui_children(
            MainLayout,
            state.left,
            [appended(first), appended(first + 1)],
        );
        state.next += 2;
    }

    if keyboard.just_pressed(KeyCode::Digit2)
        && let Some(branch) = state.branch
    {
        let parent = match child_of.get(branch) {
            Ok(child_of) if child_of.parent() == state.left => state.right,
            _ => state.left,
        };
        if let Ok(mut style) = branch_styles.get_mut(branch) {
            *style = branch_style();
        }
        commands.entity(branch).insert(ChildOf(parent));
    }

    if keyboard.just_pressed(KeyCode::Digit3)
        && let Some(branch) = state.branch
    {
        if let Ok(mut style) = branch_styles.get_mut(branch) {
            *style = branch_style().absolute().left(250.0).top(460.0);
        }
        commands.make_ui_root(MainLayout, branch);
    }

    if keyboard.just_pressed(KeyCode::Digit4)
        && let Some(branch) = state.branch.take()
    {
        commands.entity(branch).despawn();
    }
}

fn show_hierarchy(
    state: Res<HierarchyState>,
    children: Query<&Children>,
    child_of: Query<&ChildOf>,
    labels: Query<(&NodeLabel, Option<&Appended>)>,
    mut status: Single<&mut UIText, With<HierarchyStatus>>,
) {
    let left = child_order(state.left, &children, &labels);
    let right = child_order(state.right, &children, &labels);
    let branch = state
        .branch
        .map_or("despawned", |branch| match child_of.get(branch) {
            Ok(child_of) if child_of.parent() == state.left => "left panel",
            Ok(child_of) if child_of.parent() == state.right => "right panel",
            Ok(_) => "top-level",
            Err(_) => "missing",
        });

    let text = format!("Branch: {branch}\nLeft: [{left}]\nRight: [{right}]");
    if status.text != text {
        status.text = text;
    }
}

fn child_order(
    parent: Entity,
    children: &Query<&Children>,
    labels: &Query<(&NodeLabel, Option<&Appended>)>,
) -> String {
    children
        .get(parent)
        .into_iter()
        .flat_map(|children| children.iter())
        .filter_map(|entity| labels.get(entity).ok())
        .map(|(label, appended)| match appended {
            Some(Appended(number)) => format!("{} {number}", label.0),
            None => label.0.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn draw(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
