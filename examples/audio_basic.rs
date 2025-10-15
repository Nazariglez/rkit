use audio::{is_sound_playing, sound_progress};
use rkit::{
    audio::{
        Sound, SoundInstance, create_sound, create_sound_instance, play_sound, set_global_volume,
        stop_sound,
    },
    draw::create_draw_2d,
    gfx::{self, Color},
    input::{KeyCode, is_key_pressed},
    math::Vec2,
    prelude::*,
};

#[derive(Resource)]
struct AudioState {
    snd: Sound,
    ins: SoundInstance,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_setup(setup_system)
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    let snd = create_sound(include_bytes!("assets/sounds/jingles_NES00.ogg")).unwrap();
    let ins = create_sound_instance(&snd);
    cmds.insert_resource(AudioState { snd, ins });
}

fn update_system(audio: Res<AudioState>) {
    if is_key_pressed(KeyCode::Space) {
        play_sound(&audio.snd)
            .panning(0.5)
            .pitch(1.0)
            .volume(1.0)
            .repeat(true);
    }

    if is_key_pressed(KeyCode::KeyS) {
        stop_sound(&audio.snd);
    }

    if is_key_pressed(KeyCode::KeyM) {
        set_global_volume(0.2);
    }

    if is_key_pressed(KeyCode::KeyU) {
        set_global_volume(1.0);
    }
}

fn draw_system(audio: Res<AudioState>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    let is_playing = is_sound_playing(&audio.snd);
    let progress = sound_progress(&audio.snd);

    draw.text(&format!(
        "Playing: {:?}\nProgress: {:?}",
        is_playing, progress
    ))
    .position(Vec2::splat(20.0));
    gfx::render_to_frame(&draw).unwrap();
}
