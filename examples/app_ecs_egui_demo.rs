use egui_demo_lib::DemoWindows;
use rkit::{
    prelude::*,
    egui::{EguiContext, EguiPlugin},
    gfx::{self, Color},
};

fn main() -> Result<(), String> {
    App::new()
        .insert_non_send_resource(DemoWindows::default())
        .add_plugin(MainPlugins::default())
        .add_plugin(EguiPlugin::default())
        .on_render(draw_system)
        .run()
}

fn draw_system(mut ctx: ResMut<EguiContext>, mut demo: NonSendMut<DemoWindows>) {
    let edraw = ctx.clear(Color::BLACK).run(|ctx| {
        demo.ui(ctx);
    });

    gfx::render_to_frame(&edraw).unwrap();
}
