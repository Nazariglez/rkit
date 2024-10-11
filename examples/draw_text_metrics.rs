use rkit::app::window_size;
use rkit::draw::{create_draw_2d, text_metrics};
use rkit::gfx::{self, Color};
use rkit::math::Vec2;

const TEXT: &'static str = "Drawing a background with a margin of 10 pixels!. To do so, we're measuring the text before draw it.";

fn main() -> Result<(), String> {
    rkit::init().on_update(update).run()
}

fn update(state: &mut ()) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    // measure the text before draw it
    let metrics = text_metrics(TEXT).size(20.0).max_width(300.0).measure();

    // draw the background
    draw.rect(Vec2::ZERO, metrics.size + Vec2::splat(20.0)) // 10 pixeles each side
        .translate(window_size() * 0.5)
        .anchor(Vec2::splat(0.5))
        .fill_color(Color::BLACK)
        .fill()
        .stroke_color(Color::GRAY)
        .stroke(2.0);

    // let's draw the text now
    draw.text(TEXT)
        .anchor(Vec2::splat(0.5))
        .translate(window_size() * 0.5)
        .size(20.0)
        .max_width(300.0)
        .color(Color::ORANGE);

    gfx::render_to_frame(&draw).unwrap();
}
