use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::vec2;

fn main() -> Result<(), String> {
    rkit::init().update(update).run()
}

fn update() {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));
    draw.triangle(vec2(400.0, 100.0), vec2(100.0, 500.0), vec2(700.0, 500.0));
    gfx::render_to_frame(&draw).unwrap();
}
