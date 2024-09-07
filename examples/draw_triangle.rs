use rkit::draw::draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::vec2;

fn main() -> Result<(), String> {
    rkit::init().on_update(update).run()
}

fn update(s: &mut ()) {
    let mut draw = draw_2d();
    draw.clear(Color::BLACK);
    for i in 0..10 {
        let i = i as f32;
        draw.triangle(
            vec2(400.0, 100.0) + i * 10.0,
            vec2(100.0, 500.0) + i * 10.0,
            vec2(700.0, 500.0) + i * 10.0,
        )
        .color(Color::rgb(1.0 - i / 10.0, i / 20.0, 1.0 - i / 30.0));
    }
    gfx::render_to_frame(&draw).unwrap();
}
