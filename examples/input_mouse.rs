use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}, input::{MouseButton, is_mouse_btn_pressed, mouse_position}, math::Vec2};

#[derive(Resource, Default)]
struct MouseState {
    pos: Vec2,
    left: Vec<Vec2>,
    middle: Vec<Vec2>,
    right: Vec<Vec2>,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn update_system(mut state: ResMut<MouseState>) {
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
}

fn draw_system(state: Res<MouseState>, window: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    draw.circle(8.0).position(state.pos).color(Color::ORANGE);

    state.left.iter().for_each(|pos| {
        draw.circle(4.0).position(*pos).color(Color::RED);
    });

    state.middle.iter().for_each(|pos| {
        draw.circle(4.0).position(*pos).color(Color::GREEN);
    });

    state.right.iter().for_each(|pos| {
        draw.circle(4.0).position(*pos).color(Color::BLUE);
    });

    let text = format!("x: {:.0} - y: {:.0}", state.pos.x, state.pos.y);
    draw.text(&text)
        .position(window.size() * 0.5)
        .anchor(Vec2::splat(0.5))
        .size(20.0);

    gfx::render_to_frame(&draw).unwrap();
}
