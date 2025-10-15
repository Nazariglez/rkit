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
    draw.clear(Color::rgb(0.1, 0.2, 0.3));
    draw.triangle(vec2(400.0, 100.0), vec2(100.0, 500.0), vec2(700.0, 500.0));
    gfx::render_to_frame(&draw).unwrap();
}
