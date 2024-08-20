use rkit::app::*;
use rkit::input::*;
use rkit::math::*;
use std::time::{Duration, Instant};

fn main() {
    rkit::init_with(|| AppState::new())
        .with_window(WindowConfig {
            title: "Whatever".to_string(),
            size: uvec2(400, 300),
            resizable: false,
            ..Default::default()
        })
        .on_update(update)
        .on_cleanup(|_| println!("bye"))
        .run()
        .unwrap()
}

struct AppState {
    frame_count: u32,
    start_time: Instant,
    last_time: Instant,
    fps: f32,
}

impl AppState {
    fn new() -> Self {
        let now = Instant::now();
        AppState {
            frame_count: 0,
            start_time: now,
            last_time: now,
            fps: 0.0,
        }
    }
}

fn update(s: &mut AppState) {
    s.frame_count += 1;

    let current_time = Instant::now();
    let elapsed_time = current_time.duration_since(s.last_time);

    if elapsed_time >= Duration::from_secs(1) {
        s.fps = s.frame_count as f32 / elapsed_time.as_secs_f32();
        println!("FPS: {}", s.fps);
        s.frame_count = 0;
        s.last_time = current_time;
    }

    println!(
        "mpos: {:?}, is min: {:?}, max: {:?} -> Frame Count: {} -> fps: {}",
        mouse_position(),
        is_window_focused(),
        is_window_maximized(),
        s.frame_count,
        s.fps
    );

    if is_mouse_btn_pressed(MouseButton::Left) {
        close_window();
    }
}
