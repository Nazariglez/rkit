use rkit::{
    draw::create_draw_2d,
    gfx::{self, Color},
    math::vec2,
    prelude::*,
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_render(draw_system)
        .run()
}

fn draw_system() {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.path()
        .move_to(vec2(10.0, 10.0))
        .line_to(vec2(100.0, 100.0))
        .line_to(vec2(400.0, 500.0))
        .quadratic_bezier_to(vec2(440.0, 440.0), vec2(310.0, 210.0))
        .line_to(vec2(790.0, 590.0))
        .round_join()
        .color(Color::ORANGE)
        .stroke(10.0);

    gfx::render_to_frame(&draw).unwrap();
}
