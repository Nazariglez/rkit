use rkit::{
    draw::{Sprite, create_draw_2d},
    prelude::*,
    gfx::{self, Color},
    input::{MouseButton, is_mouse_btn_down},
    math::{Vec2, vec2},
    random::{self, Rng},
};

#[derive(Component)]
struct Pos(Vec2);

#[derive(Component)]
struct Speed(Vec2);

#[derive(Component)]
struct BunnyColor(Color);

#[derive(Resource)]
struct BunnySprite(Sprite);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_setup(setup_system)
        .on_update((spawn_bunnies_system, update_bunnies_system))
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    let sprite = draw::create_sprite()
        .from_image(include_bytes!("assets/bunny.png"))
        .build()
        .unwrap();

    cmds.insert_resource(BunnySprite(sprite));

    cmds.spawn((Pos(Vec2::ZERO), Speed(vec2(2.0, 10.0)), BunnyColor(Color::WHITE)));
}

fn spawn_bunnies_system(mut cmds: Commands) {
    if is_mouse_btn_down(MouseButton::Left) {
        for _ in 0..50 {
            let speed = vec2(random::range(0.0..10.0), random::range(-5.0..5.0));
            let color = Color::rgb(random::r#gen(), random::r#gen(), random::r#gen());
            cmds.spawn((Pos(Vec2::ZERO), Speed(speed), BunnyColor(color)));
        }
    }
}

fn update_bunnies_system(mut query: Query<(&mut Pos, &mut Speed)>) {
    query.par_iter_mut().for_each(|(mut pos, mut speed)| {
        pos.0 += speed.0;
        speed.0.y += 0.75;

        if pos.0.x > 800.0 {
            speed.0.x *= -1.0;
            pos.0.x = 800.0;
        } else if pos.0.x < 0.0 {
            speed.0.x *= -1.0;
            pos.0.x = 0.0;
        }

        if pos.0.y > 600.0 {
            speed.0.y *= -0.85;
            pos.0.y = 600.0;
            if random::r#gen::<bool>() {
                speed.0.y -= random::range(0.0..6.0);
            }
        } else if pos.0.y < 0.0 {
            speed.0.y = 0.0;
            pos.0.y = 0.0;
        }
    });
}

fn draw_system(query: Query<(&Pos, &BunnyColor)>, sprite: Res<BunnySprite>, time: Res<Time>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    for (pos, color) in &query {
        draw.image(&sprite.0).position(pos.0).color(color.0);
    }

    draw.text(&format!("Bunnies: {}\nFPS: {:.2}", query.iter().count(), time.fps()))
        .size(10.0)
        .position(vec2(10.0, 10.0));

    gfx::render_to_frame(&draw).unwrap();
}
