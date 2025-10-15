use rkit::prelude::*;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(FixedUpdate(1))
        .add_plugin(MainPlugins::headless())
        .on_fixed_update(1, fixed_update_system)
        .run()
}

fn fixed_update_system(time: Res<Time>) {
    println!("Game time: {}s", time.elapsed().as_secs());
}
