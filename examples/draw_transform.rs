// This is the same as 'examples/draw_transform_2d.rs' but using raw matrices (Mat3)

use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::{vec2, Mat3, Vec2};
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
    rkit::init_with(State::default).update(update).run()
}

fn update(state: &mut State) {
    state.rot = (state.rot + time::delta_f32() * 25.0).rem(360.0);

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // Push to the transformation stack a translation matrix
    draw.push_matrix(Mat3::from_translation(Vec2::new(350.0, 250.0)));

    // Calculate the matrix that we use for each object
    let translation = Mat3::from_translation(Vec2::new(30.0, 20.0));
    let rotation = Mat3::from_angle(state.rot.to_radians());
    let matrix = translation * rotation;

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
