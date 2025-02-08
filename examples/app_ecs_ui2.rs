use rkit::draw::{create_draw_2d, Draw2D};
use rkit::gfx::{self, Color};
use rkit::math::Vec2;
use rkit::prelude::*;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(WindowConfigPlugin::default().vsync(false))
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .add_systems(OnSetup, setup_system)
        .add_systems(OnUpdate, (gap_system, remove_node_system))
        .add_systems(OnRender, draw_system)
        .run()
}

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component, Deref)]
pub struct UITint(Color);

fn setup_system(mut cmds: Commands) {
    add_nodes(&mut cmds);
}

fn add_nodes(cmds: &mut Commands) {
    cmds.spawn_ui_node(
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
            Whatever,
            UIStyle::default()
                .align_items_center()
                .justify_content_center()
                .size(100.0, 100.0),
            UITint(Color::RED),
            UIRender::new::<(&UITint, &UINode), _>(draw_node),
        ),));
    })
    .entity_id();
}

#[derive(Component)]
struct Whatever;

fn gap_system(mut query: Query<&mut UIStyle>, time: Res<Time>) {
    query.iter_mut().for_each(|mut style| {
        style.gap_horizontal = match style.gap_horizontal {
            Unit::Auto => Unit::Auto,
            Unit::Pixel(px) => Unit::Pixel(px + 1.0 * time.delta_f32()),
            Unit::Relative(r) => Unit::Relative(r),
        };
        println!("{:?}", style.gap_horizontal);
    });
}

fn remove_node_system(
    mut cmds: Commands,
    query: Query<Entity, With<Whatever>>,
    key: Res<Keyboard>,
) {
    if key.just_pressed(KeyCode::Space) {
        let e = query.single();
        cmds.entity(e).despawn();
    }
}

fn draw_node(draw: &mut Draw2D, components: (&UITint, &UINode)) {
    let (tint, node) = components;
    draw.rect(Vec2::ZERO, node.size()).color(tint.0);
}

fn draw_system(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
