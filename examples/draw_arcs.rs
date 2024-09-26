use rkit::app::window_size;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::{vec2, Vec2};
use rkit::time;
use std::ops::Rem;

fn main() -> Result<(), String> {
    rkit::init().on_update(update).run()
}

fn update(s: &mut ()) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let radius = 150.0;
    let angle = (time::elapsed_f32() * 45.0).rem(360.0).to_radians();

    draw.text(&format!("{:.0}º", angle.to_degrees()))
        .anchor(window_size() * 0.5)
        .anchor(Vec2::splat(0.5));

    // draw arc
    let center = vec2(200.0, 300.0);
    let start_point = center + radius * Vec2::X;
    draw.path()
        .move_to(start_point)
        .arc(center, radius, 0.0, angle)
        .stroke_color(Color::ORANGE)
        .stroke(6.0);

    // fill semicircle
    let center = vec2(600.0, 300.0);
    draw.path()
        .move_to(center)
        .arc(center, radius, 0.0, angle * -1.0)
        .color(Color::BLUE)
        .fill();

    gfx::render_to_frame(&draw).unwrap();
}
