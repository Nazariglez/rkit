use rkit::gfx::{self, Color, Renderer};
use rkit::time;

fn main() {
    rkit::init().on_update(update).run().unwrap()
}

fn update(s: &mut ()) {
    let t = time::elapsed_f32();
    let color = Color::rgb(t.cos(), t.sin(), 1.0);

    let mut renderer = Renderer::new();
    renderer.begin_pass().clear_color(color);

    gfx::render_to_frame(&renderer).unwrap();
}
