use rkit::app::window_size;

fn main() -> Result<(), String> {
    rkit::init()
        .resize(|| log::info!("Resize {:?}", window_size()))
        .run()
}
