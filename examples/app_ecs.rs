use rkit::ecs::{App, OnCleanup, OnSetup, OnUpdate};

fn main() -> Result<(), String> {
    App::new()
        .add_systems(OnSetup, || println!("hello world"))
        .add_systems(OnUpdate, || println!("Update this shit"))
        .add_systems(OnCleanup, || println!("bye bye"))
        .run()
}
