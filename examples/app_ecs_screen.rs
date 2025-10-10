use rkit::{
    draw::create_draw_2d,
    ecs::prelude::*,
    gfx::{self, Color},
    input::KeyCode,
    macros::Screen2,
    math::{Vec2, vec2},
};

#[derive(Screen2, Debug, Hash, Clone, PartialEq, Eq, Copy)]
pub struct MenuScreen;

#[derive(Screen2, Debug, Hash, Clone, PartialEq, Eq, Copy)]
#[screen(name = "SuperDuperGameScreen")]
pub struct GameScreen;

fn screen_plugin(app: &mut App) {
    app.add_screen2::<MenuScreen>(|app| {
        app.on_render(draw_menu_system)
            .on_enter_from::<GameScreen, _>(|| println!("menu from game"))
            .on_exit_to::<GameScreen, _>(|| println!("menu to game"));
    })
    .add_screen2::<GameScreen>(|app| {
        app.on_render(draw_game_system)
            .on_enter_from::<MenuScreen, _>(|| println!("game from menu"))
            .on_exit_to::<MenuScreen, _>(|| println!("game to menu"));
    })
    .on_render(
        draw_system
            .after(MenuScreen::sys_set())
            .after(GameScreen::sys_set()),
    )
    .as_default_screen::<MenuScreen>()
    .on_update(change_screen_system);
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(screen_plugin)
        .run()
}

fn change_screen_system(mut cmds: Commands, screens: Res<Screens>, keyboard: Res<Keyboard>) {
    let is_change_requested = keyboard.just_pressed(KeyCode::Space);
    if is_change_requested {
        if screens.is_current::<MenuScreen>() {
            cmds.set_screen::<GameScreen>();
        } else {
            cmds.set_screen::<MenuScreen>();
        }
    }

    let is_clear_requested = keyboard.just_pressed(KeyCode::Escape);
    if is_clear_requested {
        cmds.clear_screen();
    }
}

fn draw_menu_system() {
    let mut draw = create_draw_2d();
    draw.clear(Color::ORANGE);

    draw.text("Menu Scene")
        .size(20.0)
        .anchor(Vec2::splat(0.5))
        .translate(vec2(400.0, 300.0));

    gfx::render_to_frame(&draw).unwrap();
}
fn draw_game_system() {
    let mut draw = create_draw_2d();
    draw.clear(Color::PURPLE);

    draw.text("Game Scene")
        .size(20.0)
        .anchor(Vec2::splat(0.5))
        .translate(vec2(400.0, 300.0));

    gfx::render_to_frame(&draw).unwrap();
}

fn draw_system(win: Res<Window>, screens: Res<Screens>) {
    let mut draw = create_draw_2d();
    match screens.current_name() {
        Some(name) => {
            draw.text(&format!("Current screen: {}", name))
                .origin(0.5)
                .translate(win.size() * 0.5 + vec2(0.0, 100.0));
        }
        None => {
            draw.clear(Color::BLACK);
            draw.text("No screen set")
                .origin(0.5)
                .translate(win.size() * 0.5);
        }
    }
    gfx::render_to_frame(&draw).unwrap();
}
