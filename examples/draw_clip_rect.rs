use rkit::{
    draw::create_draw_2d,
    gfx::{self, Color},
    input::{KeyCode, is_key_down, is_key_pressed},
    math::{Mat3, Rect, Vec2, vec2},
    prelude::*,
};

#[derive(Resource)]
struct State {
    offset: Vec2,
    nested: bool,
    empty: bool,
    rejected_rotation: bool,
    error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            nested: true,
            empty: false,
            rejected_rotation: false,
            error: None,
        }
    }
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .insert_resource(State::default())
        .on_update(update)
        .on_render(render)
        .run()
}

fn update(mut state: ResMut<State>, time: Res<Time>) {
    let movement = time.delta_f32() * 180.0;
    if is_key_down(KeyCode::KeyA) {
        state.offset.x -= movement;
    }
    if is_key_down(KeyCode::KeyD) {
        state.offset.x += movement;
    }
    if is_key_down(KeyCode::KeyW) {
        state.offset.y -= movement;
    }
    if is_key_down(KeyCode::KeyS) {
        state.offset.y += movement;
    }
    if is_key_pressed(KeyCode::KeyN) {
        state.nested = !state.nested;
    }
    if is_key_pressed(KeyCode::KeyE) {
        state.empty = !state.empty;
    }
    if is_key_pressed(KeyCode::KeyR) {
        state.rejected_rotation = !state.rejected_rotation;
    }
}

fn render(mut state: ResMut<State>, window: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));

    let panel = Rect::new(vec2(150.0, 120.0), vec2(500.0, 330.0));
    if state.rejected_rotation {
        draw.push_matrix(Mat3::from_rotation_z(0.2));
    }
    draw.push_clip(panel);
    if state.rejected_rotation {
        draw.pop_matrix();
    }

    draw.rect(vec2(70.0, 60.0) + state.offset, vec2(680.0, 440.0))
        .color(Color::rgb(0.12, 0.35, 0.65));
    draw.text("Rectangular clipping applies to shapes and text")
        .position(vec2(90.0, 260.0) + state.offset)
        .size(32.0);

    if state.nested {
        let child = if state.empty {
            Rect::new(vec2(900.0, 700.0), vec2(80.0, 80.0))
        } else {
            Rect::new(vec2(260.0, 200.0), vec2(280.0, 150.0))
        };
        draw.push_clip(child);
        draw.circle(130.0)
            .position(vec2(400.0, 275.0) + state.offset)
            .color(Color::ORANGE);
        draw.pop_clip();
    }
    draw.pop_clip();

    draw.rect(vec2(40.0, 500.0), vec2(window.size().x - 80.0, 55.0))
        .color(Color::rgb(0.15, 0.55, 0.25));
    draw.text("WASD move | N nested | E empty child | R rejected rotation")
        .position(vec2(55.0, 516.0))
        .size(20.0);
    if let Some(error) = &state.error {
        draw.text(error).position(vec2(55.0, 570.0)).size(17.0);
    }

    state.error = gfx::render_to_frame(&draw).err();
}
