use draw::create_draw_2d;
use rkit::{
    ecs::prelude::*,
    egui::{EguiContext, EguiPlugin},
    gfx::{self, Color},
    particles::*,
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(EguiPlugin::default())
        .add_plugin(ParticlesPlugin)
        .add_plugin(WindowConfigPlugin::default().maximized(true))
        .add_systems(OnSetup, setup_system)
        .add_systems(OnUpdate, update_system.before(ParticlesSysSet))
        .add_systems(OnRender, draw_system)
        .run()
}

fn setup_system(mut cmds: Commands, mut configs: ResMut<Particles>, window: Res<Window>) {
    configs.insert("my_fx".to_string(), ParticleFxConfig::default());
    cmds.spawn(
        configs
            .create_component("my_fx", window.size() * 0.5)
            .unwrap(),
    );
}

fn update_system(mut fx: Single<&mut ParticleFx>, mouse: Res<Mouse>, ctx: Res<EguiContext>) {
    fx.spawning = true;

    // FIXME: this is not working right
    if ctx.is_using_pointer() {
        return;
    }

    if mouse.is_down(MouseButton::Left) {
        fx.pos = mouse.position();
    }
}

fn draw_system(
    fx: Single<&ParticleFx>,
    mut ctx: ResMut<EguiContext>,
    mut configs: ResMut<Particles>,
) {
    // clear the backgroung
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // draw the particle first
    let fx = fx.into_inner();
    let fx_id = fx.id.clone();
    draw.particle(fx);
    gfx::render_to_frame(&draw).unwrap();

    // draw the ui
    let edraw = ctx.run(|ctx| {});

    gfx::render_to_frame(&edraw).unwrap();
}
