use rkit::app::*;

fn main() {
    rkit::init_with(init).update(update).run().unwrap()
}

fn init() -> Result<(), String> {
    set_window_title("lola");
    Ok(())
}

fn update(_s: &mut ()) {
    println!(
        "is min: {:?}, max: {:?}",
        is_window_minimized(),
        is_window_maximized()
    );
}
