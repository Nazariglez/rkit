use corelib::input::{is_key_pressed, is_mouse_btn_down, MouseButton};
use rkit::draw::{create_draw_2d, Sprite};
use rkit::ecs::prelude::*;
use rkit::ecs::{App, FixedUpdate, OnCleanup, OnFixedUpdate, OnRender, OnSetup, OnUpdate};
use rkit::gfx::Color;
use rkit::math::{vec2, Vec2};
use rkit::random::Rng;
use rkit::{gfx, random, time};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(FixedUpdate(60))
        .add_systems(OnSetup, setup_system)
        .add_systems(OnFixedUpdate(60), (update_system, add_bunnies_system))
        .add_systems(OnRender, draw_system)
        .run()
}

#[derive(Component, Debug)]
struct Pos(Vec2);
#[derive(Component, Debug)]
struct Vel(Vec2);
#[derive(Component, Debug)]
struct BunnyColor(Color);
#[derive(Resource)]
struct Random(Rng);
#[derive(Resource)]
struct Image(Sprite);
#[derive(Resource)]
struct Counter(usize);

fn setup_system(mut cmds: Commands) {
    cmds.spawn((
        Pos(Vec2::ZERO),
        Vel(vec2(2.0, 10.0)),
        BunnyColor(Color::WHITE),
    ));

    let image = draw::create_sprite()
        .from_image(include_bytes!("./assets/bunny.png"))
        .build()
        .map(|s| Image(s))
        .unwrap();
    cmds.insert_resource(image);

    let rng = Random(Rng::default());
    cmds.insert_resource(rng);

    cmds.insert_resource(Counter(0));
}

fn update_system(mut query: Query<(&mut Pos, &mut Vel)>, mut rng: ResMut<Random>) {
    query.iter_mut().for_each(|(mut pos, mut speed)| {
        pos.0 += speed.0;
        speed.0.y += 0.75;

        if pos.0.x > 800.0 {
            speed.0.x *= -1.0;
            pos.0.x = 800.0;
        } else if pos.0.x < 0.0 {
            speed.0.x *= -1.0;
            pos.0.x = 0.0
        }

        if pos.0.y > 600.0 {
            speed.0.y *= -0.85;
            pos.0.y = 600.0;
            if rng.0.gen::<bool>() {
                speed.0.y -= rng.0.range(0.0..6.0);
            }
        } else if pos.0.y < 0.0 {
            speed.0.y = 0.0;
            pos.0.y = 0.0;
        }
    });
}

fn draw_system(query: Query<(&Pos, &BunnyColor)>, img: Res<Image>, counter: Res<Counter>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    query.iter().for_each(|(Pos(pos), BunnyColor(color))| {
        draw.image(&img.0).position(*pos).color(*color);
    });

    draw.text(&format!("Bunnies: {}\nFPS: {:.2}", counter.0, time::fps()))
        .size(10.0)
        .position(vec2(10.0, 10.0));

    gfx::render_to_frame(&draw).unwrap();
}

fn add_bunnies_system(mut cmds: Commands, mut counter: ResMut<Counter>) {
    const ADD: usize = 50;
    if is_mouse_btn_down(MouseButton::Left) {
        cmds.spawn_batch((0..ADD).map(|_| {
            let speed = vec2(random::range(0.0..10.0), random::range(-5.0..5.0));
            let color = Color::rgb(random::gen(), random::gen(), random::gen());
            (Pos(Vec2::ZERO), Vel(speed), BunnyColor(color))
        }));

        counter.0 += ADD;
    }
}
