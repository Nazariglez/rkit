use draw::create_draw_2d;
use rkit::ecs::prelude::*;
use rkit::gfx;
use rkit::gfx::Color;

#[derive(Resource)]
struct MySounds {
    snd: Sound,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(AudioPlugin::default())
        .add_systems(OnSetup, setup_system)
        .add_systems(OnUpdate, update_system)
        .add_systems(OnRender, draw_system)
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
