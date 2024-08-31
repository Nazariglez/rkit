use rkit::app::*;
use rkit::gfx;
use rkit::gfx::Color;
use rkit::input::*;
use rkit::math::*;
use std::ops::Rem;

fn main() {
    rkit::init_with(|| AppState::new())
        .on_update(update)
        .on_cleanup(|_| println!("bye"))
        .run()
        .unwrap()
}

struct AppState {
    a: f32,
}

impl AppState {
    fn new() -> Self {
        AppState { a: 0.0 }
    }
}

fn update(s: &mut AppState) {
    s.a += 0.01;
    // -- Draw
    let mut renderer = gfx::Renderer::new();
    renderer
        .begin_pass()
        .clear_color(Color::from_linear_rgb(0.1, 0.2, 0.3));
    gfx::render_to_frame(&renderer).unwrap();
}
