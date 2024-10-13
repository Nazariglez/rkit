use assets::parse_asset;
use draw::Sprite;
use rkit::app::window_size;
use rkit::assets::AssetId;
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::math::Vec2;

enum State {
    Loading { asset_id: AssetId },
    World { data: Sprite },
}

fn main() -> Result<(), String> {
    rkit::init_with(|| {
        let asset_id = assets::load_asset("./examples/assets/rust-logo-512x512.png");
        State::Loading { asset_id }
    })
    .on_update(update)
    .run()
}

fn parse_sprite(id: &str, data: &[u8]) -> Result<Sprite, String> {
    log::info!("Sprite '{id}' loaded.");
    draw::create_sprite().from_image(data).build()
}

fn update(s: &mut State) {
    assets::update_assets();

    match s {
        // Loading state, we get the data if loaded and we parse it as sprite
        State::Loading { asset_id } => {
            let data = parse_asset(asset_id, parse_sprite, false).unwrap();

            if let Some(sprite) = data {
                *s = State::World { data: sprite };
            }
        }
        State::World { data } => {
            draw_world(data);
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
