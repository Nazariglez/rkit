use rkit::prelude::*;

fn main() -> Result<(), String> {
    App::new().add_plugin(MainPlugins::default()).run()
}
