use assets::parse_asset;
use draw::Sprite;
use rkit::{
    assets::AssetId,
    draw::create_draw_2d,
    prelude::*,
    gfx::{self, Color},
    math::Vec2,
};

enum LoadingState {
    Loading { asset_id: AssetId },
    World { data: Sprite },
}

#[derive(Resource)]
struct AppState(LoadingState);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_setup(setup_system)
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    let asset_id = assets::load_asset("./examples/assets/rust-logo-512x512.png");
    cmds.insert_resource(AppState(LoadingState::Loading { asset_id }));
}

fn parse_sprite(id: &str, data: &[u8]) -> Result<Sprite, String> {
    log::info!("Sprite '{id}' loaded.");
    draw::create_sprite().from_image(data).build()
}

fn update_system(mut state: ResMut<AppState>) {
    match &mut state.0 {
        LoadingState::Loading { asset_id } => {
            let data = parse_asset(asset_id, parse_sprite, false).unwrap();
            if let Some(sprite) = data {
                state.0 = LoadingState::World { data: sprite };
            }
        }
        LoadingState::World { .. } => {}
    }
}

fn draw_system(state: Res<AppState>, window: Res<Window>) {
    if let LoadingState::World { data } = &state.0 {
        let mut draw = create_draw_2d();
        draw.clear(Color::BLACK);
        draw.image(data)
            .position(window.size() * 0.5)
            .anchor(Vec2::splat(0.5));

        gfx::render_to_frame(&draw).unwrap();
    }
}
