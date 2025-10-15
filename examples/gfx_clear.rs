use rkit::{
    gfx::{self, Color, Renderer},
    prelude::*,
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_render(render_system)
        .run()
}

fn render_system(time: Res<Time>) {
    let t = time.elapsed_f32();
    let color = Color::rgb(t.cos(), t.sin(), 1.0);

    let mut renderer = Renderer::new();
    renderer.begin_pass().clear_color(color);

    gfx::render_to_frame(&renderer).unwrap();
}
