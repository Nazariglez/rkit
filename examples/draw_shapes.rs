use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::vec2;

fn main() -> Result<(), String> {
    rkit::init().on_update(update).run()
}

fn update(s: &mut ()) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    draw.line(vec2(20.0, 30.0), vec2(780.0, 30.0)).width(4.0);

    draw.triangle(vec2(100.0, 100.0), vec2(150.0, 200.0), vec2(200.0, 100.0))
        .fill_color(Color::YELLOW)
        .fill()
        .stroke_color(Color::GRAY)
        .stroke(6.0);

    draw.rect(vec2(500.0, 100.0), vec2(200.0, 150.0))
        .fill_color(Color::GREEN)
        .fill()
        .stroke_color(Color::WHITE)
        .stroke(15.0);

    draw.ellipse(vec2(400.0, 300.0), vec2(50.0, 100.0))
        .color(Color::RED)
        .rotate(45.0f32.to_degrees());

    draw.circle(40.0)
        .position(vec2(600.0, 450.0))
        .fill_color(Color::BLUE)
        .fill()
        .stroke_color(Color::WHITE)
        .stroke(5.0);

    draw.rect(vec2(100.0, 250.0), vec2(150.0, 100.0))
        .corner_radius(20.0)
        .color(Color::ORANGE)
        .stroke(15.0);

    draw.star(10, 80.0, 40.0)
        .position(vec2(150.0, 480.0))
        .fill_color(Color::PINK)
        .fill()
        .stroke_color(Color::MAGENTA)
        .stroke(6.0);

    draw.polygon(5, 50.0)
        .position(vec2(350.0, 150.0))
        .color(Color::WHITE)
        .stroke(8.0);

    draw.polygon(8, 80.0)
        .position(vec2(350.0, 450.0))
        .fill_color(Color::WHITE)
        .fill()
        .stroke_color(Color::ORANGE)
        .stroke(8.0);

    gfx::render_to_frame(&draw).unwrap();
}
