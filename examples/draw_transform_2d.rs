// This is the same as 'examples/draw_transform.rs' but using the Transform2D helper

use draw::Transform2D;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::{vec2, Vec2};
use rkit::time;
use std::ops::Rem;

const COLORS: [Color; 8] = [
    Color::WHITE,
    Color::MAGENTA,
    Color::ORANGE,
    Color::RED,
    Color::YELLOW,
    Color::AQUA,
    Color::MAROON,
    Color::PINK,
];

#[derive(Default)]
struct State {
    rot: f32,
}

fn main() -> Result<(), String> {
    rkit::init_with(State::default).on_update(update).run()
}

fn update(state: &mut State) {
    state.rot = (state.rot + time::delta_f32() * 25.0).rem(360.0);

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // Push to the transformation stack a translation matrix
    draw.push_matrix(
        Transform2D::new()
            .set_translation(vec2(350.0, 250.0))
            .as_mat3(),
    );

    // Calculate the matrix that we use for each object
    let matrix = Transform2D::new()
        .set_translation(vec2(30.0, 20.0))
        .set_rotation(state.rot.to_radians())
        .as_mat3();

    for (i, c) in COLORS.iter().enumerate() {
        let n = (i * 7) as f32;
        let size = 100.0 - n;

        // Push to the stack the same matrix on each draw
        // The new matrices will be multiplied by the latest on the stack
        draw.push_matrix(matrix);

        // Create a rect
        draw.rect(vec2(0.0, 0.0), Vec2::splat(size))
            // Using different color for each rect
            .color(*c);
    }

    // Reset the transformation stack
    draw.clear_matrix_stack();

    gfx::render_to_frame(&draw).unwrap();
}
