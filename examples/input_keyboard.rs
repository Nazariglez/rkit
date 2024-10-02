use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::input::{is_key_down, keys_released, KeyCode};
use rkit::math::{vec2, Vec2};
use rkit::time;

const MOVE_SPEED: f32 = 100.0;

struct State {
    pos: Vec2,
    last_key: Option<KeyCode>,
}

impl State {
    fn new() -> Self {
        Self {
            pos: vec2(400.0, 300.0),
            last_key: None,
        }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).on_update(update).run()
}

fn update(state: &mut State) {
    if let Some(last_key) = keys_released().iter().last() {
        state.last_key = Some(last_key);
    }

    let movement = MOVE_SPEED * time::delta_f32();
    if is_key_down(KeyCode::KeyW) {
        state.pos.y -= movement;
    }

    if is_key_down(KeyCode::KeyA) {
        state.pos.x -= movement;
    }

    if is_key_down(KeyCode::KeyS) {
        state.pos.y += movement;
    }

    if is_key_down(KeyCode::KeyD) {
        state.pos.x += movement;
    }

    draw(state);
}

fn draw(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.circle(50.0)
        .translate(s.pos)
        .anchor(Vec2::splat(0.5))
        .color(Color::RED);

    draw.text("Use WASD to move the circle")
        .position(Vec2::splat(10.0));

    if let Some(key) = &s.last_key {
        draw.text(&format!("Last key: {key:?}"))
            .position(vec2(10.0, 560.0));
    }

    gfx::render_to_frame(&draw).unwrap();
}
