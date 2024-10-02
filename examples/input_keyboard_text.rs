use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::input::text_pressed;
use rkit::math::{vec2, Vec2};

struct State {
    msg: String,
}

impl State {
    fn new() -> Self {
        Self { msg: String::new() }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).on_update(update).run()
}

fn update(state: &mut State) {
    let text = text_pressed();
    text.iter().for_each(|t| {
        state.msg.push_str(t);
    });

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.text("Type anything:")
        .position(Vec2::splat(10.0))
        .color(Color::YELLOW);

    draw.text(&state.msg)
        .position(vec2(20.0, 50.0))
        .max_width(760.0)
        .color(Color::WHITE);

    gfx::render_to_frame(&draw).unwrap();
}
