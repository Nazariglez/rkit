use rkit::app::window_size;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};

const RADIUS: f32 = 150.0;

fn main() -> Result<(), String> {
    rkit::init().update(update).run()
}

fn update(s: &mut ()) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    draw.circle(RADIUS)
        .position(window_size() * 0.5 - RADIUS)
        .color(Color::MAGENTA)
        .fill()
        .stroke_color(Color::WHITE)
        .stroke(20.0);

    gfx::render_to_frame(&draw).unwrap();
}
