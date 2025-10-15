use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}, math::Vec2};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_render(draw_system)
        .run()
}

fn draw_system(window: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.text("Hello world")
        .position(window.size() * 0.5)
        .anchor(Vec2::splat(0.5))
        .color(Color::ORANGE)
        .size(48.0);

    gfx::render_to_frame(&draw).unwrap();
}
