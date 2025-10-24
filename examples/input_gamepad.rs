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

fn update_system(gamepads: Res<Gamepads>) {
    gamepads.iter().for_each(|gamepad| {
        let pressed = gamepad.pressed_buttons();
        pressed.iter().for_each(|btn| {
            log::info!(
                "Gamepad: {:?} - pressed: {:?}",
                gamepad.slot(),
                gamepad.button_name(btn)
            );
        });
        let released = gamepad.released_buttons();
        released.iter().for_each(|btn| {
            log::info!(
                "Gamepad: {:?} - released: {:?}",
                gamepad.slot(),
                gamepad.button_name(btn)
            );
        });
        let down = gamepad.down_buttons();
        down.iter().for_each(|btn| {
            log::info!(
                "Gamepad: {:?} - down: {:?}",
                gamepad.slot(),
                gamepad.button_name(btn)
            );
        });
        let axis = gamepad.axis_states();
        axis.iter()
            .filter(|(_axis, strength)| *strength != 0.0)
            .for_each(|(axis, strength)| {
                log::info!(
                    "Gamepad: {:?} - axis: {:?} - movement: {:?}",
                    gamepad.slot(),
                    gamepad.axis_name(axis),
                    strength
                );
            });
    });
}

fn draw_system() {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    gfx::render_to_frame(&draw).unwrap();
}
