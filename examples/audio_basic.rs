use audio::{create_sound, play_sound, Sound};
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::input::{is_key_pressed, KeyCode};
use rkit::math::vec2;

struct State {
    snd: Sound,
}

impl State {
    fn new() -> Self {
        let snd = create_sound(include_bytes!("assets/sounds/jingles_NES00.ogg")).unwrap();
        Self { snd }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).on_update(update).run()
}

fn update(s: &mut State) {
    if is_key_pressed(KeyCode::Space) {
        play_sound(&s.snd);
    }
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));
    draw.triangle(vec2(400.0, 100.0), vec2(100.0, 500.0), vec2(700.0, 500.0));
    gfx::render_to_frame(&draw).unwrap();
}
