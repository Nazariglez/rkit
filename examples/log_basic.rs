use rkit::app::LogConfig;

fn main() -> Result<(), String> {
    rkit::init_with(setup).with_logs(LogConfig::trace()).run()
}

fn setup() {
    log::trace!("Trace? Yes, trace log"); // not in wasm32
    log::debug!("Hello, this is a debug log...");
    log::info!("And this is a info log!");
    log::warn!("I'm warning you");
    log::error!("I'm an error, I told you...");
}
