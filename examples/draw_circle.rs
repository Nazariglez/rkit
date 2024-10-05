use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::vec2;

fn main() -> Result<(), String> {
    rkit::init().on_update(update).run()
}

fn update(s: &mut ()) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.rect(vec2(100.0, 100.0), vec2(600.0, 400.0));
    gfx::render_to_frame(&draw).unwrap();
}
