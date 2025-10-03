use draw::{BaseCam2D, ScreenMode};
use rkit::app::window_size;
use rkit::draw::{Camera2D, create_draw_2d};
use rkit::gfx::{self, Color};
use rkit::math::{Vec2, vec2};
use rkit::prelude::*;

/// Identify the layout and the entities that belongs to it
#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component)]
struct Cam(Camera2D);

#[derive(Component)]
#[require(UIPointer)]
struct Highlight {
    color: Color,
    base: Color,
}

// const RESOULTION: Vec2 = Vec2::new(640.0, 380.0);
const RESOULTION: Vec2 = Vec2::new(800.0, 600.0);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .on_setup(setup_system)
        .on_update((update_system, highlight_system).chain())
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    let mut cam = Camera2D::new(Vec2::ONE, ScreenMode::AspectFit(RESOULTION));
    cam.set_position(RESOULTION * 0.5);
    cam.update();
    cmds.spawn(Cam(cam));

    // ui
    cmds.spawn_ui_node(
        MainLayout,
        (
            UIContainer {
                bg_color: Some(Color::ORANGE),
                ..Default::default()
            },
            UIStyle::default()
                .size_full()
                .align_items_center()
                .justify_content_center(),
        ),
    )
    .with_children(|cmd| {
        cmd.add((
            UIContainer {
                bg_color: Some(Color::WHITE),
                ..Default::default()
            },
            UIStyle::default().size(300.0, 100.0),
            UIPointer::default(),
            Highlight {
                color: Color::GREEN,
                base: Color::WHITE,
            },
        ));
    });
}

/// we need to set the layout's size or pass a camera to know the root's size
fn update_system(
    ui_cam: Single<&mut Cam>,
    mut layout: ResMut<UILayout<MainLayout>>,
    win: Res<Window>,
) {
    let mut cam = ui_cam.into_inner();
    cam.0.set_size(win.size());
    cam.0.update();

    layout.set_camera(&cam.0);
}

fn draw_system(world: &mut World) {
    let mut draw = create_draw_2d();

    // you must apply the same camera when drawing
    let cam = world.query::<&Cam>().single(world);
    draw.set_camera(&cam.0);

    // clear and draw layout
    draw.clear(Color::BLACK);

    draw_ui_layout::<MainLayout>(&mut draw, world);

    draw.rect(Vec2::ZERO, RESOULTION)
        .color(Color::BLUE)
        .stroke(8.0);

    gfx::render_to_frame(&draw).unwrap();
}

fn highlight_system(mut query: Query<(&mut UIContainer, &UIPointer, &Highlight)>) {
    for (mut container, pointer, highlight) in &mut query {
        if pointer.just_enter() {
            container.bg_color = Some(highlight.color);
        } else if pointer.just_exit() {
            container.bg_color = Some(highlight.base);
        }
    }
}
