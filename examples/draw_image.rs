use rkit::{draw::{Sprite, create_draw_2d}, prelude::*, gfx::{self, Color}}};

#[derive(Resource)]
struct FerrisSprite(Sprite);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_setup(setup_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    let sprite = draw::create_sprite()
        .from_image(include_bytes!("assets/ferris.png"))
        .build()
        .unwrap();
    cmds.insert_resource(FerrisSprite(sprite));
}

fn draw_system(sprite: Res<FerrisSprite>, window: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    draw.image(&sprite.0)
        .position(window.size() * 0.5 - sprite.0.size() * 0.5);

    gfx::render_to_frame(&draw).unwrap();
}
