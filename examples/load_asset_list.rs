use assets::{parse_asset, AssetList};
use draw::{Font, Sprite};
use rkit::app::window_size;
use rkit::assets::AssetId;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::Vec2;

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
    rkit::init_with(setup).on_update(update).run()
}

fn setup() -> State {
    let list = AssetList::new(&[
        "./examples/assets/bunny.png",
        "./examples/assets/ferris.png",
        "./examples/assets/Ubuntu-B.ttf",
        "./examples/assets/data.txt",
    ])
        .with_extension_parser("png", parse_sprite);
        // .with_extension_parser("ttf", parse_font);
    State::Loading { list }
}

fn parse_sprite(id: &str, data: &[u8]) -> Result<Sprite, String> {
    log::info!("Sprite '{id}' loaded.");
    draw::create_sprite().from_image(data).build()
}

// fn parse_font(id: &str, data: &[u8]) -> Result<Font, String> {
//     log::info!("Font '{id}' loaded.");
//     let d = data.to_vec();
//     draw::create_font(&d).build()
// }


fn update(s: &mut State) {
    assets::update_assets();

    match s {
        // Loading state, we get the data if loaded and we parse it as sprite
        State::Loading { list } => {
            let data = list.parse(|d| Ok(())).unwrap();

            if let Some(list) = data {
                // *s = State::World { data: sprite };
            }
        }
        // If the sprite is loaded we draw it
        State::World { assets } => {
            // draw_world(data);
        }
    }
}

fn draw_world(sprite: &Sprite) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.image(sprite)
        .translate(window_size() * 0.5)
        .anchor(Vec2::splat(0.5));

    gfx::render_to_frame(&draw).unwrap();
}
