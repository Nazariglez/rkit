use rkit::{
    draw::{create_draw_2d, text_metrics},
    gfx::{self, Color},
    math::Vec2,
    prelude::*,
};

const TEXT: &str = "Drawing a background with a margin of 10 pixels!. To do so, we're measuring the text before draw it.";

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_render(draw_system)
        .run()
}

fn draw_system(window: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    let metrics = text_metrics(TEXT).size(20.0).max_width(300.0).measure();

    draw.rect(Vec2::ZERO, metrics.size + Vec2::splat(20.0))
        .position(window.size() * 0.5)
        .anchor(Vec2::splat(0.5))
        .fill_color(Color::BLACK)
        .fill()
        .stroke_color(Color::GRAY)
        .stroke(2.0);

    draw.text(TEXT)
        .position(window.size() * 0.5)
        .anchor(Vec2::splat(0.5))
        .size(20.0)
        .max_width(300.0)
        .color(Color::ORANGE);

    gfx::render_to_frame(&draw).unwrap();
}
