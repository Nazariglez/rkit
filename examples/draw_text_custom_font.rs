use rkit::{draw::{self, Font, create_draw_2d}, prelude::*, gfx::{self, Color}, math::vec2};

#[derive(Resource)]
struct CustomFont(Font);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_setup(setup_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    let font = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();
    cmds.insert_resource(CustomFont(font));
}

fn draw_system(font: Res<CustomFont>, window: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let pos = window.size() * 0.5;
    let offset = vec2(0.0, 10.0);
    draw.text("Using Ubuntu-B font.")
        .font(&font.0)
        .position(pos - offset)
        .color(Color::ORANGE)
        .size(48.0)
        .anchor(vec2(0.5, 1.0));

    draw.text("Using Arcade-Legacy (default) font.")
        .position(pos + offset)
        .color(Color::YELLOW)
        .size(10.0)
        .anchor(vec2(0.5, 0.0));

    gfx::render_to_frame(&draw).unwrap();
}
