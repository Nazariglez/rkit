use rkit::app::WindowConfig;

fn main() -> Result<(), String> {
    rkit::init_with(|| ())
        .with_window(
            WindowConfig::default()
                .title("Window Icon")
                .window_icon(include_bytes!("./assets/rust-logo-512x512.png"))
                .taskbar_icon("examples/assets/rust-logo-512x512.png"),
        )
        .run()
}
