use rkit::app::*;

fn main() {
    rkit::init().on_update(update).run().unwrap()
}

fn update(_s: &mut ()) {
    println!(
        "is min: {:?}, max: {:?}",
        is_window_focused(),
        is_window_maximized()
    );
}
