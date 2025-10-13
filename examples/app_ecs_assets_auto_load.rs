use rkit::{
    assets2::*,
    draw::*,
    gfx::{self, Color},
    macros::assets,
    math::vec2,
    prelude::*,
};

#[assets(
    root = "./examples/assets",
    types(
        png: Sprite, 
        txt: String
    ),
    embed = true,
)]
mod assets {}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(AssetsPlugin::default())
        .on_setup(setup_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut loader: ResMut<AssetLoader>) {
    loader.add_parser("txt", text_parser);
    loader.add_parser("png", sprite_parser);

    loader.auto_load::<assets::Assets>();
}

fn draw_system(win: Res<Window>, assets: Option<Res<assets::Assets>>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    match assets {
        Some(assets) => {
            draw.image(&assets.ferris)
                .translate(win.size() * 0.5)
                .origin(vec2(0.5, 1.0));
            draw.text(&assets.data)
                .size(10.0)
                .translate(win.size() * 0.5 + vec2(0.0, 80.0))
                .origin(0.5);
        }
        None => {
            draw.text("Assets not loaded")
                .size(10.0)
                .translate(win.size() * 0.5)
                .origin(0.5);
        }
    }

    gfx::render_to_frame(&draw).unwrap();
}

fn text_parser(data: In<AssetData>) -> Result<String, String> {
    Ok(String::from_utf8(data.data.clone()).unwrap())
}

fn sprite_parser(data: In<AssetData>) -> Result<Sprite, String> {
    draw::create_sprite().from_image(&data.data).build()
}
