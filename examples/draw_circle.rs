use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}};

const RADIUS: f32 = 150.0;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_render(draw_system)
        .run()
}

fn draw_system(window: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    draw.circle(RADIUS)
        .position(window.size() * 0.5 - RADIUS)
        .color(Color::MAGENTA)
        .fill()
        .stroke_color(Color::WHITE)
        .stroke(20.0);

    gfx::render_to_frame(&draw).unwrap();
}
