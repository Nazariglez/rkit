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

    let mdelta = mouse_motion_delta();
    let wdelta = mouse_wheel_delta();

    if is_mouse_moving() || is_mouse_scrolling() {
        println!("{:?} - {:?}", mdelta, wdelta);
    }

    // println!(
    //     "mpos: {:?}, Frame Count: {} -> fps: {} -- {:?}, motion_delta: {:?}, wheel_delta: {:?}",
    //     mouse_position(),
    //     s.frame_count,
    //     s.fps,
    //     are_mouse_btns_down(&[MouseButton::Left, MouseButton::Right, MouseButton::Middle]),
    //     mouse_motion_delta(),
    //     mouse_wheel_delta()
    // );

    // if is_mouse_btn_pressed(MouseButton::Left) {
    //     close_window();
    // }
}
