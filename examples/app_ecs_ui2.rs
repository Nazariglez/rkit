use rkit::draw::{create_draw_2d, Draw2D};
use rkit::gfx::{self, Color};
use rkit::math::Vec2;
use rkit::prelude::*;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(AddMainPlugins::default())
        .add_plugin(AddWindowConfigPlugin::default().vsync(false))
        .add_plugin(UIPlugin)
        .add_systems(OnSetup, setup_system)
        .add_systems(OnRender, draw_system)
        .run()
}

#[derive(Component, Clone, Copy)]
struct MainLayout;

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn apply(self, app: App) -> App {
        app.add_resource(UILayout::<MainLayout>::default())
            .add_systems(OnPostUpdate, compute_layout_system)
    }
}

fn compute_layout_system(
    mut query: Query<&mut UINode>,
    mut layout: ResMut<UILayout<MainLayout>>,
    win: Res<Window>,
) {
    layout.set_size(win.size()); // TODO: fixme
    let updated = layout.update(None);
    if updated {
        query
            .iter_mut()
            .for_each(|mut node| layout.update_node(&mut node));
    }
}

#[derive(Component, Deref)]
pub struct UITint(Color);

fn setup_system(mut cmds: Commands) {
    add_nodes(&mut cmds);
}

fn add_nodes(cmds: &mut Commands) {
    cmds.spawn_ui(
        MainLayout,
        (
            UIStyle::default()
                .flex_row()
                .size_full()
                .align_items_center()
                .gap_x(20.0)
                .justify_content_center(),
            UITint(Color::WHITE),
            UIRender::new::<(&UITint, &UINode), _>(draw_node),
        ),
    )
    .with_children(|cmd| {
        cmd.add(((
            UIStyle::default()
                .align_items_center()
                .justify_content_center()
                .size(100.0, 100.0),
            UITint(Color::ORANGE),
            UIRender::new::<(&UITint, &UINode), _>(draw_node),
        ),))
            .with_children(|cmd| {
                cmd.add((
                    UIStyle::default().size(40.0, 20.0),
                    UITint(Color::BLUE),
                    UIRender::new::<(&UITint, &UINode), _>(draw_node),
                ));
            });

        cmd.add(((
            UIStyle::default()
                .align_items_center()
                .justify_content_center()
                .size(100.0, 100.0),
            UITint(Color::RED),
            UIRender::new::<(&UITint, &UINode), _>(draw_node),
        ),));
    });
}

fn draw_node(draw: &mut Draw2D, components: (&UITint, &UINode)) {
    let (tint, node) = components;
    draw.rect(Vec2::ZERO, node.size()).color(tint.0);
}

fn draw_system(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw_ui::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
