use rkit::draw::{create_draw_2d, Draw2D};
use rkit::gfx::{self, Color};
use rkit::math::Vec2;
use rkit::prelude::*;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .add_systems(OnSetup, setup_system)
        .add_systems(OnRender, draw_system)
        .run()
}

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component, Deref)]
pub struct Tint(Color);

fn setup_system(mut cmds: Commands) {
    let root = cmds
        .spawn_ui_node(
            MainLayout,
            (
                UIStyle::default()
                    .flex_row()
                    .size_full()
                    .align_items_center()
                    .gap_x(20.0)
                    .justify_content_center(),
                Tint(Color::WHITE),
                UIRender::new::<(&Tint, &UINode), _>(draw_node),
            ),
        )
        .with_children(|cmd| {
            cmd.add(((
                UIStyle::default()
                    .align_items_center()
                    .justify_content_center()
                    .size(100.0, 100.0),
                Tint(Color::ORANGE),
                UIRender::new::<(&Tint, &UINode), _>(draw_node),
            ),))
                .with_children(|cmd| {
                    cmd.add((
                        UIStyle::default().size(40.0, 20.0),
                        Tint(Color::BLUE),
                        UIRender::new::<(&Tint, &UINode), _>(draw_node),
                    ));
                });

            cmd.add(((
                UIStyle::default()
                    .align_items_center()
                    .justify_content_center()
                    .size(100.0, 100.0),
                Tint(Color::RED),
                UIRender::new::<(&Tint, &UINode), _>(draw_node),
            ),));
        })
        .entity_id();

    let child = cmds
        .spawn_ui_node(
            MainLayout,
            (
                UIStyle::default()
                    .align_items_center()
                    .justify_content_center()
                    .size(10.0, 50.0),
                Tint(Color::GREEN),
                UIRender::new::<(&Tint, &UINode), _>(draw_node),
            ),
        )
        .entity_id();

    cmds.add_ui_child(MainLayout, root, child);
}

fn draw_node(draw: &mut Draw2D, components: (&Tint, &UINode)) {
    let (tint, node) = components;
    draw.rect(Vec2::ZERO, node.size()).color(tint.0);
}

fn draw_system(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
