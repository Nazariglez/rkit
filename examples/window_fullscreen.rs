use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}, input::{KeyCode, is_key_pressed}, math::{Vec2, vec2}}};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn update_system() {
    if is_key_pressed(KeyCode::Space) {
        // TODO: Implement fullscreen toggle in ECS
    }
}

fn draw_system(window: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let (txt, color) = ("Fullscreen toggle", Color::ORANGE);

    draw.text(txt)
        .position(window.size() * 0.5)
        .anchor(vec2(0.5, 1.0))
        .color(color)
        .size(20.0);

    draw.text("Press SPACE to toggle fullscreen mode")
        .position(window.size() * 0.5 + Vec2::Y * 10.0)
        .anchor(vec2(0.5, 0.0));

    gfx::render_to_frame(&draw).unwrap();
}
