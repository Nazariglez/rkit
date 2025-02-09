use corelib::input::{is_mouse_btn_down, MouseButton};
use rkit::draw::{create_draw_2d, Sprite};
use rkit::ecs::prelude::*;
use rkit::gfx::Color;
use rkit::math::{vec2, Vec2};
use rkit::random::Rng;
use rkit::{gfx, random};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(WindowConfigPlugin::default().title("BunnyMark"))
        .add_systems(OnSetup, setup_system)
        .add_systems(OnUpdate, (update_system, add_bunnies_system))
        .add_systems(OnRender, draw_system)
        .run()
}

#[derive(Component, Debug, Deref)]
struct Pos(Vec2);
#[derive(Component, Debug, Deref)]
struct Vel(Vec2);
#[derive(Component, Debug, Deref)]
struct BunnyColor(Color);

#[derive(Resource, Deref)]
struct Random(Rng);
#[derive(Resource, Deref)]
struct Image(Sprite);
#[derive(Resource, Deref)]
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
        .map(Image)
        .unwrap();
    cmds.insert_resource(image);

    let rng = Random(Rng::default());
    cmds.insert_resource(rng);

    cmds.insert_resource(Counter(0));
}

fn update_system(mut query: Query<(&mut Pos, &mut Vel)>, mut rng: ResMut<Random>) {
    query.iter_mut().for_each(|(mut pos, mut speed)| {
        pos.0 += speed.0;
        speed.y += 0.75;

        if pos.x > 800.0 {
            speed.x *= -1.0;
            pos.x = 800.0;
        } else if pos.x < 0.0 {
            speed.x *= -1.0;
            pos.x = 0.0
        }

        if pos.y > 600.0 {
            speed.y *= -0.85;
            pos.y = 600.0;
            if rng.gen::<bool>() {
                speed.y -= rng.range(0.0..6.0);
            }
        } else if pos.y < 0.0 {
            speed.y = 0.0;
            pos.y = 0.0;
        }
    });
}

fn draw_system(
    query: Query<(&Pos, &BunnyColor)>,
    img: Res<Image>,
    counter: Res<Counter>,
    time: Res<Time>,
) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    query.iter().for_each(|(Pos(pos), BunnyColor(color))| {
        draw.image(&img).position(*pos).color(*color);
    });

    draw.text(&format!("Bunnies: {}\nFPS: {:.2}", counter.0, time.fps()))
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
