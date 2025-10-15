use rkit::{
    assets2::*,
    draw::*,
    gfx::{self, Color},
    macros::assets,
    math::vec2,
    prelude::*,
};

#[derive(Debug, Clone)]
pub struct GameMetadata {
    pub title: String,
    pub version: String,
}

#[assets(
    root = "./examples/assets",
    types(png: Sprite, ttf: Font, txt: String),
    custom(metadata: GameMetadata = parse_game_metadata)
)]
mod assets {}

fn main() -> Result<(), String> {
    let a_plugin = AssetsPlugin::default()
        .add_parser("png", sprite_parser)
        .add_parser("txt", text_parser)
        .add_parser("ttf", font_parser);

    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(a_plugin)
        .on_setup(setup_system)
        .on_render(draw_system)
        .run()
}
fn setup_system(mut loader: ResMut<AssetLoader>) {
    loader.auto_load::<assets::Assets>();
}

fn draw_system(win: Res<Window>, assets: Option<Res<assets::Assets>>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    match assets {
        Some(assets) => {
            // Draw some loaded sprites
            draw.image(&assets.ferris)
                .translate(win.size() * vec2(0.3, 0.3))
                .origin(vec2(0.5, 0.5))
                .scale(0.5);

            draw.image(&assets.bunny)
                .translate(win.size() * vec2(0.7, 0.3))
                .origin(vec2(0.5, 0.5))
                .scale(0.3);

            // Display custom metadata
            draw.text(&format!("Game: {}", assets.metadata.title))
                .font(&assets.ubuntu_b)
                .size(24.0)
                .color(Color::WHITE)
                .translate(win.size() * vec2(0.5, 0.1))
                .origin(vec2(0.5, 0.5));

            draw.text(&format!("Version: {}", assets.metadata.version))
                .font(&assets.ubuntu_b)
                .size(16.0)
                .color(Color::GRAY)
                .translate(win.size() * vec2(0.5, 0.15))
                .origin(vec2(0.5, 0.5));

            // Show loaded data file content
            draw.text(&format!("Data: {}", assets.data))
                .font(&assets.ubuntu_b)
                .size(12.0)
                .color(Color::GREEN)
                .translate(win.size() * vec2(0.5, 0.8))
                .origin(vec2(0.5, 0.5));
        }
        None => {
            // Loading screen
            draw.text("Loading assets...")
                .size(24.0)
                .color(Color::WHITE)
                .translate(win.size() * 0.5)
                .origin(vec2(0.5, 0.5));
        }
    }

    gfx::render_to_frame(&draw).unwrap();
}

// Custom parser function
fn parse_game_metadata(
    _world: &mut World,
    _loader: &mut AssetLoader,
) -> Result<Option<GameMetadata>, String> {
    Ok(Some(GameMetadata {
        title: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

// Asset parser functions
fn sprite_parser(data: In<AssetData>) -> Result<Sprite, String> {
    draw::create_sprite().from_image(&data.data).build()
}

fn text_parser(data: In<AssetData>) -> Result<String, String> {
    Ok(String::from_utf8(data.data.clone()).unwrap())
}

fn font_parser(data: In<AssetData>) -> Result<Font, String> {
    draw::create_font(&data.data).build()
}
