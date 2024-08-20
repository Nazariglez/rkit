use rkit::*;
use rkit::math::*;
use rkit::app::*;

fn main() {
    rkit::init()
        .with_window(WindowConfig {
            title: "Whatever".to_string(),
            size: uvec2(400, 300),
            resizable: false,
            ..Default::default()
        })
        .on_update(update)
        .run()
        .unwrap()
}

fn update(_s: &mut ()) {
    println!(
        "is min: {:?}, max: {:?}",
        is_window_focused(),
        is_window_maximized()
    );
}
