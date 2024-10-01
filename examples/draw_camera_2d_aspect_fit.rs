use draw::ScreenMode;
use rkit::app::window_size;
use rkit::draw::create_draw_2d;
use rkit::draw::Camera2D;
use rkit::gfx::{self, Color};
use rkit::math::{vec2, Vec2};

// TODO fix debug mode crash because glam_assert

const WORK_SIZE: Vec2 = Vec2::new(800.0, 600.0);

struct State {
    cam: Camera2D,
}

impl State {
    fn new() -> Self {
        let size = window_size();
        let cam = Camera2D::new(size, ScreenMode::AspectFit(WORK_SIZE));
        Self { cam }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).on_update(update).run()
}

fn update(s: &mut State) {
    // set camera size and position
    let size = window_size();
    s.cam.set_size(size);
    s.cam.set_position(WORK_SIZE * 0.5);
    s.cam.update();

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // The camera must be set before any drawing
    draw.set_camera(&s.cam);

    // draw
    draw.rect(Vec2::ZERO, WORK_SIZE)
        .color(Color::ORANGE)
        .stroke(10.0);

    draw.triangle(vec2(400.0, 100.0), vec2(100.0, 500.0), vec2(700.0, 500.0));

    gfx::render_to_frame(&draw).unwrap();
}
