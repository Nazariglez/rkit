use draw::create_draw_2d;
use rkit::{prelude::*, gfx::{self, Color}};

#[derive(Resource)]
struct MySounds {
    snd: Sound,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(AudioPlugin::default())
        .on_setup(setup_system)
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands, mut audio: ResMut<Audio>) {
    let snd = audio
        .create_sound(include_bytes!("./assets/sounds/jingles_NES00.ogg"))
        .unwrap();
    cmds.insert_resource(MySounds { snd });
}

fn update_system(keyboard: Res<Keyboard>, sounds: Res<MySounds>, mut audio: ResMut<Audio>) {
    if keyboard.just_pressed(KeyCode::Space) {
        audio.play(&sounds.snd);
    }
}

fn draw_system() {
    let mut draw = create_draw_2d();
    draw.clear(Color::ORANGE);

    gfx::render_to_frame(&draw).unwrap();
}
