use rkit::{
    draw::create_draw_2d,
    gfx::{self, Color},
    math::{Vec2, vec2},
    particles::*,
    prelude::*,
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(ParticlesPlugin)
        .on_setup(setup_system)
        .on_update(update_system.before(ParticlesSysSet))
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands, mut configs: ResMut<Particles>, window: Res<Window>) {
    // configs.insert(
    //     "basic".to_string(),
    //     ParticleFxConfig {
    //         emitters: vec![EmitterConfig {
    //             id: todo!(),
    //             def: todo!(),
    //             sprites: todo!(),
    //         }],
    //         is_local: todo!(),
    //     },
    // );

    // configs.insert(
    //     "burst".to_string(),
    //     ParticleFxConfig {
    //         emitters: vec![EmitterConfig {
    //             id: todo!(),
    //             def: todo!(),
    //             sprites: todo!(),
    //         }],
    //         is_local: todo!(),
    //     },
    // );
    //
    // cmds.spawn(
    //     configs
    //         .create_component("basic", window.size() * 0.5)
    //         .unwrap(),
    // );
}

fn update_system(mouse: Res<Mouse>, mut particles: Query<&mut ParticleFx>) {
    particles.iter_mut().for_each(|mut p| {
        p.pos = mouse.position();
        p.spawning = mouse.is_down(MouseButton::Left);
    });
}

fn draw_system(particles: Query<&ParticleFx>, time: Res<Time>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    particles.iter().for_each(|fx| {
        draw.particle(fx);
    });

    draw.text(&format!("{:.2}", time.fps()));
    gfx::render_to_frame(&draw).unwrap();
}
