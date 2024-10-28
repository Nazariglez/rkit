use rkit::time;

fn main() -> Result<(), String> {
    rkit::init().fixed_update(1.0 / 2.0, fixed_update).run()
}

fn fixed_update() {
    println!(
        "Run each half second. (game_time: {:.3})",
        time::elapsed_f32()
    );
}
