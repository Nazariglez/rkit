use rkit::{
    assets2::*,
    draw::*,
    gfx::{self, Color},
    prelude::*,
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(AssetsPlugin)
        .on_setup(setup_system)
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut loader: ResMut<AssetLoader>) {
    // let id =loader.load("./examples/assets/bunny.png");
    loader.load_list("my_list", &["./examples/assets/bunny.png"]);
    loader.add_parser("txt", text_parser);
    loader.add_parser("png", sprite_parser);

    // loader.load("./examples/assets/data.txt");
    // loader.load("./examples/assets/bunny.png");
    loader.load_bytes("./examples/assets/data.txt", b"hello world");
}

fn update_system(mut cmds: Commands, mut loader: ResMut<AssetLoader>) {
    let is_loaded = loader.is_loaded("./examples/assets/data.txt");
    if is_loaded {
        let data = loader.take::<String>("./examples/assets/data.txt").unwrap();
        println!("data: {data:?}");
    }
}

fn draw_system(mut loader: ResMut<AssetLoader>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    if let Some(sprite) = loader.get::<Sprite>("./examples/assets/bunny.png") {
        draw.image(&sprite);
    }

    gfx::render_to_frame(&draw).unwrap();
}

fn text_parser(data: In<AssetData>) -> Result<String, String> {
    Ok(String::from_utf8(data.data.clone()).unwrap())
}

fn sprite_parser(data: In<AssetData>) -> Result<Sprite, String> {
    draw::create_sprite().from_image(&data.data).build()
}
