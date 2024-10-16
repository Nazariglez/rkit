use rkit::app::window_size;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::vec2;
use rkit::time;
use std::f32::consts::PI;

fn main() -> Result<(), String> {
    rkit::init().update(update).run()
}

fn update() {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // Time value for the animation
    let time = time::elapsed_f32();
    let w_size = window_size();

    // Dynamically change amplitude and frequency over time
    let amplitude = 100.0 * (time * 0.5).sin();
    let frequency = 2.5 * (time * 0.3).cos();
    let speed = 4.0;
    let wave_length = w_size.x;
    let offset = w_size.y * 0.5;

    {
        // Start drawing the wave
        let mut path = draw.path();
        path.move_to(vec2(0.0, offset)); // Starting point

        for n in (0..=wave_length as usize).step_by(10) {
            let x = n as f32;
            let y = offset
                + amplitude * (frequency * (x / wave_length * PI * 2.0) + time * speed).sin();
            path.line_to(vec2(x, y));
        }

        path.color(Color::rgba_u8(200, 32, 176, 255))
            .round_join()
            .stroke(5.0);
    }

    gfx::render_to_frame(&draw).unwrap();
}
