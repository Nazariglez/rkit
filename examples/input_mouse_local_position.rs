use draw::Transform2D;
use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}, input::mouse_position, math::{Vec2, vec2}};

const RECT_SIZE: Vec2 = Vec2::new(400.0, 300.0);

#[derive(Resource)]
struct Rotation(f32);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_setup(setup_system)
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    cmds.insert_resource(Rotation(0.0));
}

fn update_system(mut rot: ResMut<Rotation>, time: Res<Time>) {
    rot.0 += time.delta_f32();
}

fn draw_system(rot: Res<Rotation>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    draw.push_matrix(
        Transform2D::new()
            .set_size(RECT_SIZE)
            .set_translation(vec2(200.0, 150.0))
            .set_pivot(Vec2::splat(0.5))
            .set_rotation(rot.0)
            .updated_mat3(),
    );

    let local_pos = draw.screen_to_local(mouse_position());
    let color = rect_color(local_pos, RECT_SIZE);

    draw.rect(Vec2::ZERO, RECT_SIZE).color(color);

    draw.pop_matrix();

    gfx::render_to_frame(&draw).unwrap();
}

// Set the color to red if the mouse is inside the bounds of the matrix
fn rect_color(local: Vec2, size: Vec2) -> Color {
    let Vec2 {
        x: width,
        y: height,
    } = size;
    let in_bounds = local.x >= 0.0 && local.x <= width && local.y >= 0.0 && local.y <= height;
    if in_bounds { Color::RED } else { Color::WHITE }
}
