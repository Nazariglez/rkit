use rkit::prelude::*;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(FixedUpdate(2))
        .add_plugin(MainPlugins::default())
        .on_fixed_update(2, fixed_update_system)
        .run()
}

fn fixed_update_system(time: Res<Time>) {
    println!(
        "Run each half second. (game_time: {:.3})",
        time.elapsed_f32()
    );
}
