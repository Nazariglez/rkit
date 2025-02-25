use assets::{AssetList, AssetMap};
use draw::{Font, Sprite};
use rkit::app::window_size;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::{Vec2, vec2};

struct Assets {
    tex1: Sprite,
    tex2: Sprite,
    font1: Font,
    data: Vec<u8>,
}

enum State {
    Loading { list: AssetList },
    World { assets: Assets },
}

fn main() -> Result<(), String> {
    rkit::init_with(setup).update(update).run()
}

fn setup() -> State {
    let list = AssetList::new(&[
        "./examples/assets/bunny.png",
        "./examples/assets/ferris.png",
        "./examples/assets/Ubuntu-B.ttf",
        "./examples/assets/data.txt",
    ])
    .with_extension_parser("png", parse_sprite)
    .with_extension_parser("ttf", parse_font);
    State::Loading { list }
}

fn parse_sprite(id: &str, data: &[u8]) -> Result<Sprite, String> {
    draw::create_sprite().from_image(data).build()
}

fn parse_font(id: &str, data: &[u8]) -> Result<Font, String> {
    draw::create_font(data).build()
}

fn parse_assets(map: &AssetMap) -> Result<Assets, String> {
    Ok(Assets {
        tex1: map.get("./examples/assets/bunny.png")?,
        tex2: map.get("./examples/assets/ferris.png")?,
        font1: map.get("./examples/assets/Ubuntu-B.ttf")?,
        data: map.get("./examples/assets/data.txt")?,
    })
}

fn update(s: &mut State) {
    match s {
        // Loading state, we get the data if loaded and we parse it as sprite
        State::Loading { list } => {
            draw_loading();

            let data = list.parse(parse_assets).unwrap();

            if let Some(list) = data {
                *s = State::World { assets: list };
            }
        }
        // If the sprite is loaded we draw it
        State::World { assets } => {
            draw_world(assets);
        }
    }
}

fn draw_loading() {
    let mut draw = draw::create_draw_2d();
    draw.clear(Color::BLACK);
    draw.text("Loading...")
        .anchor(Vec2::splat(0.5))
        .translate(window_size() * 0.5)
        .size(24.0);

    gfx::render_to_frame(&draw).unwrap();
}

fn draw_world(assets: &Assets) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // draw loaded image
    draw.image(&assets.tex2)
        .translate(window_size() * 0.5)
        .anchor(Vec2::splat(0.5));

    // draw loaded text file with loaded font
    let txt = String::from_utf8_lossy(&assets.data);
    draw.text(&txt)
        .font(&assets.font1)
        .anchor(Vec2::splat(0.5))
        .translate(window_size() * vec2(0.5, 0.8))
        .size(24.0);

    gfx::render_to_frame(&draw).unwrap();
}
