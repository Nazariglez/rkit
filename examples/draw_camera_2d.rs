use draw::ScreenMode;
use rkit::app::{window_size, WindowConfig};
use rkit::draw::create_draw_2d;
use rkit::draw::Camera2D;
use rkit::gfx::{self, Color};
use rkit::input::{is_key_down, is_key_pressed, KeyCode};
use rkit::math::{uvec2, vec2, Vec2};
use rkit::time;
use std::ops::Rem;

const WORK_SIZE: Vec2 = Vec2::splat(400.0);
const MODES: [ScreenMode; 4] = [
    ScreenMode::Normal,
    ScreenMode::Fill(WORK_SIZE),
    ScreenMode::AspectFill(WORK_SIZE),
    ScreenMode::AspectFit(WORK_SIZE),
];

struct State {
    cam: Camera2D,
    player_pos: Vec2,
    mode_idx: usize,
}

impl State {
    fn new() -> Self {
        let size = window_size();
        let cam = Camera2D::new(size, ScreenMode::Normal);
        Self {
            cam,
            player_pos: WORK_SIZE * 0.5,
            mode_idx: 0,
        }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new)
        .with_window(WindowConfig::default().size(600, 500))
        .update(update)
        .run()
}

fn update(s: &mut State) {
    let speed = 100.0;
    let dt = time::delta_f32();
    if is_key_down(KeyCode::KeyA) {
        s.player_pos.x -= speed * dt;
    } else if is_key_down(KeyCode::KeyD) {
        s.player_pos.x += speed * dt;
    }

    if is_key_down(KeyCode::KeyW) {
        s.player_pos.y -= speed * dt;
    } else if is_key_down(KeyCode::KeyS) {
        s.player_pos.y += speed * dt;
    }

    if is_key_pressed(KeyCode::Space) {
        s.mode_idx = (s.mode_idx + 1).rem(MODES.len());
        s.cam.set_screen_mode(MODES[s.mode_idx]);
    }

    // Update camera size to the window's size and position to player's size
    s.cam.set_size(window_size());
    s.cam.set_position(s.player_pos);
    s.cam.update();

    draw(s);
}

fn draw(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // The camera must be set before any drawing
    draw.set_camera(&s.cam);

    // draw
    let character_size = Vec2::splat(50.0);
    draw.rect(Vec2::ZERO, character_size)
        .anchor(Vec2::splat(0.5))
        .translate(s.player_pos)
        .fill_color(Color::MAGENTA)
        .fill()
        .stroke_color(Color::PINK)
        .stroke(3.0);

    draw.text(&format!("{:.0},{:.0}", s.player_pos.x, s.player_pos.y))
        .translate(s.player_pos - Vec2::Y * character_size * 0.6)
        .anchor(vec2(0.5, 1.0))
        .size(8.0);

    draw.rect(Vec2::ZERO, WORK_SIZE)
        .color(Color::GRAY)
        .stroke(2.0);

    gfx::render_to_frame(&draw).unwrap();

    // UI - pass
    let mut draw = create_draw_2d();
    draw.text("The working resolution is 400x400. Depending on the screen mode the content of the window should adapt to that. Try changing the mode and resizing the window.")
        .anchor(vec2(1.0, 0.0))
        .translate(vec2(window_size().x - 10.0, 10.0))
        .h_align_right()
        .max_width(300.0)
        .size(9.0);

    draw.text("WASD to move character.\nSPACE to change mode")
        .translate(Vec2::splat(10.0))
        .size(10.0);

    let bounds = draw.last_text_bounds();
    draw.text(&format!("Mode: {:?}", s.cam.screen_mode()))
        .translate(vec2(bounds.min().x, bounds.max().y + 10.0))
        .size(10.0);

    let cam_bounds = s.cam.bounds();
    draw.text(&format!(
        "Visible area: min({:.0},{:.0}) max({:.0},{:.0})",
        cam_bounds.min().x,
        cam_bounds.min().y,
        cam_bounds.max().x,
        cam_bounds.max().y
    ))
    .translate(vec2(10.0, window_size().y - 10.0))
    .anchor(vec2(0.0, 1.0))
    .size(10.0);

    gfx::render_to_frame(&draw).unwrap();
}
