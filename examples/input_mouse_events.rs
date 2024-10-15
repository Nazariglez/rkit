use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::input::{
    hide_cursor, is_cursor_locked, is_cursor_on_screen, is_cursor_visible, is_key_pressed,
    is_mouse_btn_down, is_mouse_moving, is_mouse_scrolling, lock_cursor, mouse_btns_pressed,
    mouse_btns_released, mouse_motion_delta, mouse_position, mouse_wheel_delta, show_cursor,
    unlock_cursor, KeyCode, MouseButton,
};
use rkit::math::{vec2, Vec2};
use rkit::ring_buffer::RingBuffer;

struct State {
    ring_buffer: RingBuffer<String, 5>,
}

impl State {
    fn new() -> Self {
        let empty = String::new();
        Self {
            ring_buffer: RingBuffer::new([
                empty.clone(),
                empty.clone(),
                empty.clone(),
                empty.clone(),
                empty.clone(),
            ]),
        }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).update(update).run()
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let mut text = String::from("- Info:\n");

    let pos = mouse_position();
    text.push_str(&format!("Position: {:.0},{:.0}\n", pos.x, pos.y));

    let motion_delta = mouse_motion_delta();
    text.push_str(&format!(
        "Motion delta: {:.0},{:.0}\n",
        motion_delta.x, motion_delta.y
    ));

    let is_moving = is_mouse_moving();
    text.push_str(&format!("Moving: {:?}\n", is_moving));

    let is_scrolling = is_mouse_scrolling();
    text.push_str(&format!("Scrolling: {:?}\n", is_scrolling));

    let wheel_delta = mouse_wheel_delta();
    text.push_str(&format!(
        "Wheel delta: {:.0},{:.0}\n",
        wheel_delta.x, wheel_delta.y
    ));

    let is_cursor_on_screen = is_cursor_on_screen();
    text.push_str(&format!("Cursor on screen: {:?}\n", is_cursor_on_screen));

    let is_cursor_locked = is_cursor_locked();
    text.push_str(&format!("Cursor locked: {:?}\n", is_cursor_locked));

    let is_cursor_visible = is_cursor_visible();
    text.push_str(&format!("Cursor visible: {:?}\n", is_cursor_visible));

    text.push_str("\n- Buttons: \n");
    let btns = [
        MouseButton::Left,
        MouseButton::Middle,
        MouseButton::Right,
        MouseButton::Forward,
        MouseButton::Forward,
        MouseButton::Unknown,
    ];
    btns.into_iter().for_each(|btn| {
        text.push_str(&format!(
            "{:?} is down: {:?}\n",
            btn,
            is_mouse_btn_down(btn)
        ))
    });

    text.push_str("\n- Events: \n");
    let pressed = mouse_btns_pressed();
    if !pressed.is_empty() {
        state
            .ring_buffer
            .push(format!("Pressed: {:?}", pressed.iter().collect::<Vec<_>>()));
    }

    let released = mouse_btns_released();
    if !released.is_empty() {
        state.ring_buffer.push(format!(
            "Released: {:?}",
            released.iter().collect::<Vec<_>>()
        ));
    }

    state.ring_buffer.iter().enumerate().for_each(|(i, s)| {
        text.push_str(&format!("{}. {}\n", i + 1, s));
    });

    draw.text(&text).position(vec2(20.0, 10.0));

    // actions
    if is_key_pressed(KeyCode::Space) {
        if is_cursor_visible {
            hide_cursor();
        } else {
            show_cursor();
        }
    }

    if is_key_pressed(KeyCode::KeyL) {
        if is_cursor_locked {
            unlock_cursor();
        } else {
            lock_cursor();
        }
    }

    draw.text("Press SPACE to show/hide cursor and L to lock/unlock it.")
        .translate(vec2(400.0, 550.0))
        .anchor(Vec2::splat(0.5))
        .size(10.0);

    gfx::render_to_frame(&draw).unwrap();
}
