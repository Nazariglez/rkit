use draw::Transform2D;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::input::mouse_position;
use rkit::math::{vec2, Vec2};
use rkit::time;

const RECT_SIZE: Vec2 = Vec2::new(400.0, 300.0);

#[derive(Default)]
struct State {
    rot: f32,
}

fn main() -> Result<(), String> {
    rkit::init_with(State::default).update(update).run()
}

fn update(state: &mut State) {
    state.rot += time::delta_f32();

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // Matrix for a rotating rectangle
    draw.push_matrix(
        Transform2D::new()
            .set_size(RECT_SIZE)
            .set_translation(vec2(200.0, 150.0))
            .set_pivot(Vec2::splat(0.5))
            .set_rotation(state.rot)
            .updated_mat3(),
    );

    // local position from mouse position
    let local_pos = draw.screen_to_local(mouse_position());
    // assign the red color if the mouse is on top
    let color = rect_color(local_pos, RECT_SIZE);

    // draw the rectangle
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
    if in_bounds {
        Color::RED
    } else {
        Color::WHITE
    }
}
