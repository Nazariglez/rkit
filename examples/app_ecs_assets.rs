use rkit::assets2::*;
use rkit::prelude::*;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(AssetsPlugin)
        .on_setup(setup_system)
        .run()
}

fn setup_system(mut loader: ResMut<AssetLoader>) {
    // let id =loader.load("./examples/assets/bunny.png");
    loader.load_list(
        "my_list",
        &["./examples/assets/bunny.png", "./examples/assets/data.txt"],
    );
}
