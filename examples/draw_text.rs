use rkit::app::window_size;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::Vec2;

fn main() -> Result<(), String> {
    rkit::init().update(update).run()
}

fn update(s: &mut ()) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.text("Hello world")
        .position(window_size() * 0.5)
        .anchor(Vec2::splat(0.5))
        .color(Color::ORANGE)
        .size(48.0);

    gfx::render_to_frame(&draw).unwrap();
}
