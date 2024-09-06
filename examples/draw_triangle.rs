use rkit::draw::draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::vec2;

fn main() -> Result<(), String> {
    rkit::init().on_update(update).run()
}

fn update(s: &mut ()) {
    let mut draw = draw_2d();
    draw.clear(Color::BLACK);
    draw.triangle(vec2(400.0, 100.0), vec2(100.0, 500.0), vec2(700.0, 500.0));
    gfx::render_to_frame(&draw).unwrap();
}
