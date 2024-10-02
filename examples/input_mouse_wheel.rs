use rkit::app::window_size;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::input::{is_mouse_scrolling, mouse_wheel_delta};
use rkit::math::{vec2, Vec2};

struct State {
    pos: Vec2,
}

impl State {
    fn new() -> Self {
        Self {
            pos: vec2(400.0, 300.0),
        }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).on_update(update).run()
}

fn update(state: &mut State) {
    let w_size = window_size();

    if is_mouse_scrolling() {
        let delta = mouse_wheel_delta();
        state.pos = (state.pos + delta).min(w_size).max(Vec2::ZERO);
    }

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.text("Scroll with your mouse's wheel or touchpad")
        .translate(w_size * 0.5)
        .anchor(Vec2::splat(0.5));

    draw.circle(30.0)
        .translate(state.pos)
        .color(Color::RED)
        .anchor(Vec2::splat(0.5));

    gfx::render_to_frame(&draw).unwrap();
}
