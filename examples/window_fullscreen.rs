use rkit::app::{is_window_fullscreen, toggle_fullscreen, window_size};
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::input::{KeyCode, is_key_pressed};
use rkit::math::{Vec2, vec2};

fn main() -> Result<(), String> {
    rkit::init().update(update).run()
}

fn update() {
    if is_key_pressed(KeyCode::Space) {
        toggle_fullscreen();
    }

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let (txt, color) = if is_window_fullscreen() {
        ("Fullscreen enabled!", Color::ORANGE)
    } else {
        ("Fullscreen disabled", Color::GRAY)
    };

    draw.text(txt)
        .translate(window_size() * 0.5)
        .anchor(vec2(0.5, 1.0))
        .color(color)
        .size(20.0);

    draw.text("Press SPACE to toggle fullscreen mode")
        .translate(window_size() * 0.5 + Vec2::Y * 10.0)
        .anchor(vec2(0.5, 0.0));

    gfx::render_to_frame(&draw).unwrap();
}
