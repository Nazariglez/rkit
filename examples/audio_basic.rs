use audio::{clean_audio_manager, is_sound_playing, sound_progress};
use rkit::audio::{
    create_sound, create_sound_instance, play_sound, set_global_volume, stop_sound, Sound,
    SoundInstance,
};
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::input::{is_key_pressed, KeyCode};
use rkit::math::{vec2, Vec2};
use rkit::time;

struct State {
    snd: Sound,
    ins: SoundInstance,
}

impl State {
    fn new() -> Self {
        let snd = create_sound(include_bytes!("assets/sounds/jingles_NES00.ogg")).unwrap();
        let ins = create_sound_instance(&snd);
        Self { snd, ins }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).on_update(update).run()
}

fn update(s: &mut State) {
    if is_key_pressed(KeyCode::Space) {
        play_sound(&s.snd);
    }
    if is_key_pressed(KeyCode::KeyS) {
        stop_sound(&s.snd);
    }
    if is_key_pressed(KeyCode::KeyM) {
        set_global_volume(0.2);
    }
    if is_key_pressed(KeyCode::KeyU) {
        set_global_volume(1.0);
    }

    let t = time::elapsed_f32();
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    let is_playing = is_sound_playing(&s.snd);
    let progress = sound_progress(&s.snd);

    draw.text(&format!(
        "Playing: {:?}\nProgress: {:?}",
        is_playing, progress
    ))
    .position(Vec2::splat(20.0));
    gfx::render_to_frame(&draw).unwrap();

    clean_audio_manager();
}
