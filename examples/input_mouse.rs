use rkit::app::window_size;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::input::{is_mouse_btn_pressed, mouse_position, MouseButton};
use rkit::math::{vec2, Vec2};

#[derive(Default)]
struct State {
    pos: Vec2,
    left: Vec<Vec2>,
    middle: Vec<Vec2>,
    right: Vec<Vec2>,
}

fn main() -> Result<(), String> {
    rkit::init_with(State::default).on_update(update).run()
}

fn update(state: &mut State) {
    // get mouse cursor position here
    let pos = mouse_position();

    if is_mouse_btn_pressed(MouseButton::Left) {
        state.left.push(pos);
    }

    if is_mouse_btn_pressed(MouseButton::Middle) {
        state.middle.push(pos);
    }

    if is_mouse_btn_pressed(MouseButton::Right) {
        state.right.push(pos);
    }

    state.pos = pos;

    draw_ui(state);
}

fn draw_ui(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // Draw cursor
    draw.circle(8.0).position(state.pos).color(Color::ORANGE);

    // Draw left clicks
    state.left.iter().for_each(|pos| {
        draw.circle(4.0).position(*pos).color(Color::RED);
    });

    // Draw middle clicks
    state.middle.iter().for_each(|pos| {
        draw.circle(4.0).position(*pos).color(Color::GREEN);
    });

    // Draw right clicks
    state.right.iter().for_each(|pos| {
        draw.circle(4.0).position(*pos).color(Color::BLUE);
    });

    // Draw position
    let text = format!("x: {:.0} - y: {:.0}", state.pos.x, state.pos.y);
    draw.text(&text)
        .position(window_size() * 0.5)
        .size(20.0)
        .h_align_center()
        .v_align_middle();

    gfx::render_to_frame(&draw).unwrap();
}
