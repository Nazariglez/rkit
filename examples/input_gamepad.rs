use rkit::{
    draw::create_draw_2d,
    gfx::{self, Color},
    prelude::*,
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(GamepadPlugin)
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn update_system(keyboard: Res<Keyboard>, gamepads: Res<Gamepads>) {
    let check_pads = keyboard.just_pressed(KeyCode::Space);
    if check_pads {
        gamepads.iter_connected().for_each(|gamepad| {
            println!(
                "Gamepad connected? {:?} - slot: {:?} name: '{:?}' uuid: '{:?}'",
                gamepad.is_connected(),
                gamepad.slot(),
                gamepad.name(),
                gamepad.uuid()
            );
        });
    }
}

fn draw_system() {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    gfx::render_to_frame(&draw).unwrap();
}
