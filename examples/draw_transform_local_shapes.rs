use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::{Vec2, vec2};
use rkit::time;

#[derive(Default)]
struct State {
    rot: f32,
}

fn main() -> Result<(), String> {
    rkit::init_with(State::default).update(update).run()
}

fn update(state: &mut State) {
    state.rot += time::delta_f32() * 25.0;

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let n = state.rot * 0.1;

    draw.rect(Vec2::ZERO, Vec2::splat(100.0))
        .translate(vec2(110.0 + n.sin() * 100.0, 10.0));

    draw.rect(Vec2::ZERO, Vec2::splat(100.0))
        .color(Color::AQUA)
        .pivot(Vec2::splat(0.5))
        // Helper to pivot from a point using degrees
        .rotation(state.rot.to_radians())
        // Matrix translation
        .translate(Vec2::splat(200.0));

    draw.circle(20.0)
        .color(Color::ORANGE)
        // Matrix translation
        .translate(vec2(500.0, 320.0))
        // Helper to scale from a point
        .anchor(Vec2::splat(0.5))
        .scale(vec2(2.0 + n.sin(), 2.0 + n.cos()));

    draw.rect(Vec2::ZERO, Vec2::splat(100.0))
        .color(Color::MAGENTA)
        .translate(vec2(200.0, 400.0))
        .rotation(state.rot * 0.5f32.to_radians());

    gfx::render_to_frame(&draw).unwrap();
}
