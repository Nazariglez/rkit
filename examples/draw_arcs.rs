use rkit::app::window_size;
use rkit::draw::draw_2d;
use rkit::gfx::{self, Color};

fn main() -> Result<(), String> {
    rkit::init().on_update(update).run()
}

fn update(s: &mut ()) {
    let mut draw = draw_2d();
    draw.clear(Color::BLACK);

    let center = window_size() * 0.5;
    let radius = 150.0;

    draw.path()
        .move_to(center)
        .arc(
            center,
            radius,
            std::f32::consts::PI,
            std::f32::consts::PI / 2.0,
        ) // A quarter circle
        .stroke(2.0); // You can specify stroke width here

    gfx::render_to_frame(&draw).unwrap();
}
