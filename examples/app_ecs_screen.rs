use rkit::draw::create_draw_2d;
use rkit::ecs::prelude::*;
use rkit::gfx;
use rkit::gfx::Color;
use rkit::input::{is_key_pressed, KeyCode};
use rkit::macros::Screen;
use rkit::math::{vec2, Vec2};

#[derive(Screen, Debug, Hash, Clone, PartialEq, Eq)]
enum Scene {
    Menu,
    Game,
}

struct InitScenes;
impl Plugin for InitScenes {
    fn apply(self, app: &mut App) -> &mut App {
        app.with_screen(Scene::Menu)
            .add_screen_systems(Scene::Menu, OnRender, draw_menu_system)
            .add_screen_systems(Scene::Game, OnRender, draw_game_system)
            .add_systems(OnUpdate, change_screen_system.run_if(is_change_requested))
    }
}
fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(InitScenes)
        .run()
}

fn is_change_requested() -> bool {
    is_key_pressed(KeyCode::Space)
}

fn change_screen_system(mut cmds: Commands, scene: Res<Scene>) {
    let to = match *scene {
        Scene::Menu => Scene::Game,
        Scene::Game => Scene::Menu,
    };

    cmds.queue(ChangeScreen(to));
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
