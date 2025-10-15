use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}, math::vec2};
use std::f32::consts::PI;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_render(draw_system)
        .run()
}

fn draw_system(window: Res<Window>, time: Res<Time>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let elapsed = time.elapsed_f32();
    let w_size = window.size();

    let amplitude = 100.0 * (elapsed * 0.5).sin();
    let frequency = 2.5 * (elapsed * 0.3).cos();
    let speed = 4.0;
    let wave_length = w_size.x;
    let offset = w_size.y * 0.5;

    {
        let mut path = draw.path();
        path.move_to(vec2(0.0, offset));

        for n in (0..=wave_length as usize).step_by(10) {
            let x = n as f32;
            let y = offset
                + amplitude * (frequency * (x / wave_length * PI * 2.0) + elapsed * speed).sin();
            path.line_to(vec2(x, y));
        }

        path.color(Color::rgba_u8(200, 32, 176, 255))
            .round_join()
            .stroke(5.0);
    }

    gfx::render_to_frame(&draw).unwrap();
}
