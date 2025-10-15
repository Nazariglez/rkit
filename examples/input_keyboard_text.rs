use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}, input::text_pressed, math::{Vec2, vec2}}};

#[derive(Resource, Default)]
struct TextState {
    msg: String,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn update_system(mut state: ResMut<TextState>) {
    let text = text_pressed();
    text.iter().for_each(|t| {
        state.msg.push_str(t);
    });
}

fn draw_system(state: Res<TextState>) {
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
