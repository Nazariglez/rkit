use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}, math::vec2};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_render(draw_system)
        .run()
}

fn draw_system() {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.rect(vec2(100.0, 100.0), vec2(600.0, 400.0));
    gfx::render_to_frame(&draw).unwrap();
}
