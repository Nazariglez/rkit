use draw::ScreenMode;
use rkit::app::{window_size, WindowConfig};
use rkit::draw::create_draw_2d;
use rkit::draw::Camera2D;
use rkit::gfx::{self, Color};
use rkit::input::{is_key_down, is_key_pressed, KeyCode};
use rkit::math::{uvec2, vec2, Vec2};
use rkit::time;
use std::fmt::format;
use std::ops::Rem;

const WORK_SIZE: Vec2 = Vec2::splat(400.0);
const MODES: [ScreenMode; 4] = [
    ScreenMode::Basic,
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
        let cam = Camera2D::new(size);
        Self {
            cam,
            player_pos: size * 0.5,
            mode_idx: 0,
        }
    }
}

fn main() -> Result<(), String> {
    let win = WindowConfig {
        size: WORK_SIZE.as_uvec2() + uvec2(100, 150),
        vsync: true,
        ..Default::default()
    };
    rkit::init_with(State::new)
        .with_window(win)
        .on_update(update)
        .run()
}

fn update(s: &mut State) {
    // s.cam.set_position(s.cam.position() + vec2(10.0, 0.0) * time::delta_f32());

    let speed = 100.0;
    let dt = time::delta_f32();
    if is_key_down(KeyCode::ArrowLeft) {
        s.player_pos.x -= speed * dt;
    } else if is_key_down(KeyCode::ArrowRight) {
        s.player_pos.x += speed * dt;
    }

    if is_key_down(KeyCode::ArrowUp) {
        s.player_pos.y -= speed * dt;
    } else if is_key_down(KeyCode::ArrowDown) {
        s.player_pos.y += speed * dt;
    }

    if is_key_pressed(KeyCode::Space) {
        s.mode_idx = (s.mode_idx + 1).rem(MODES.len());
        s.cam.set_screen_mode(MODES[s.mode_idx]);
        println!("HERE {:?}", s.cam.screen_mode());
    }

    s.cam.set_size(window_size());
    // s.cam.set_position(s.player_pos);
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

    gfx::render_to_frame(&draw).unwrap();

    // UI
    let mut draw = create_draw_2d();
    draw.text("The working resolution is 400x400. Depending on the screen mode the content of the window should adapt to that. Try changing the mode and resizing the window.")
        .anchor(vec2(1.0, 0.0))
        .translate(vec2(window_size().x - 10.0, 10.0))
        .h_align_right()
        .max_width(300.0)
        .size(9.0);

    draw.text("Arrows to move character.\nSPACE to change mode")
        .translate(Vec2::splat(10.0))
        .size(10.0);

    let bounds = draw.last_text_bounds();
    draw.text(&format!("Mode: {:?}", s.cam.screen_mode()))
        .translate(vec2(bounds.min().x, bounds.max().y + 10.0))
        .size(10.0);

    gfx::render_to_frame(&draw).unwrap();
}