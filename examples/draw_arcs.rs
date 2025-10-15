use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}, math::{Vec2, vec2}};
use std::ops::Rem;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_render(draw_system)
        .run()
}

fn draw_system(window: Res<Window>, time: Res<Time>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let radius = 150.0;
    let angle = (time.elapsed_f32() * 45.0).rem(360.0).to_radians();

    draw.text(&format!("{:.0}º", angle.to_degrees()))
        .position(window.size() * 0.5)
        .anchor(Vec2::splat(0.5));

    let center = vec2(200.0, 300.0);
    let start_point = center + radius * Vec2::X;
    draw.path()
        .move_to(start_point)
        .arc(center, radius, 0.0, angle)
        .stroke_color(Color::ORANGE)
        .stroke(6.0);

    let center = vec2(600.0, 300.0);
    draw.path()
        .move_to(center)
        .arc(center, radius, 0.0, angle * -1.0)
        .color(Color::BLUE)
        .fill();

    gfx::render_to_frame(&draw).unwrap();
}
