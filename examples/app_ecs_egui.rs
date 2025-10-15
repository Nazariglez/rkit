use rkit::{
    prelude::*,
    egui::{EguiContext, EguiPlugin},
    gfx::{self, Color},
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(EguiPlugin::default())
        .on_render(draw_system)
        .run()
}

fn draw_system(mut ctx: ResMut<EguiContext>) {
    let edraw = ctx.clear(Color::BLACK).run(|ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello world!");
            if ui.button("Click me").clicked() {
                // take some action here
                println!("click?");
            }
        });
    });

    gfx::render_to_frame(&edraw).unwrap();
}
