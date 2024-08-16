use app::window;

fn main() {
    rkit::init_with(init).update(update).run().unwrap()
}

fn init() -> Result<(), String> {
    window::init()?;

    Ok(())
}

fn update(_s: &mut ()) {
    println!("update")
}
