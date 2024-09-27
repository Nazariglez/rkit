use rkit::app::window_size;
use rkit::draw::create_draw_2d;
use rkit::draw::Camera2D;
use rkit::gfx::{self, Color};
use rkit::input::{is_key_down, KeyCode};
use rkit::math::{vec2, Vec2};
use rkit::time;

struct State {
    cam: Camera2D,
    player_pos: Vec2,
}

impl State {
    fn new() -> Self {
        let size = window_size();
        let cam = Camera2D::new(size);
        Self {
            cam,
            player_pos: size * 0.5,
        }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).on_update(update).run()
}

fn update(s: &mut State) {
    // s.cam.set_position(s.cam.position() + vec2(10.0, 0.0) * time::delta_f32());
    s.cam.update();

    let speed = 100.0;
    let dt = time::delta_f32();
    if is_key_down(KeyCode::ArrowLeft) {
        s.player_pos.x -= speed * dt;
    } else if is_key_down(KeyCode::ArrowRight) {
        s.player_pos.x += speed * dt;
    }

    draw(s);
}

fn draw(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // The camera must be set before any drawing
    draw.set_camera(&s.cam);

    // draw
    draw.rect(Vec2::ZERO, Vec2::splat(50.0))
        .anchor(Vec2::splat(0.5))
        .translate(s.player_pos);

    gfx::render_to_frame(&draw).unwrap();
}
