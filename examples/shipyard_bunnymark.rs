use rkit::draw::{Sprite, create_draw_2d};
use rkit::gfx::Color;
use rkit::input::{MouseButton, is_mouse_btn_down};
use rkit::math::{Vec2, vec2};
use rkit::random::Rng;
use rkit::{draw, gfx, time};
use shipyard::{
    Component, EntitiesViewMut, IntoIter, Unique, UniqueView, UniqueViewMut, View, ViewMut, World,
};

#[derive(Component, Debug)]
struct Pos(Vec2);
#[derive(Component, Debug)]
struct Vel(Vec2);
#[derive(Component, Debug)]
struct BunnyColor(Color);

#[derive(Unique)]
struct Resources {
    rng: Rng,
    sprite: Sprite,
}

fn main() -> Result<(), String> {
    rkit::init_with(setup).update(update).run()
}

fn setup() -> World {
    let mut world = World::new();
    world.add_entity((
        Pos(Vec2::ZERO),
        Vel(vec2(2.0, 10.0)),
        BunnyColor(Color::WHITE),
    ));

    let sprite = draw::create_sprite()
        .from_image(include_bytes!("./assets/bunny.png"))
        .build()
        .unwrap();

    let rng = Rng::default();
    world.add_unique(Resources { sprite, rng });

    world
}

fn update_bunnies(mut pos: ViewMut<Pos>, mut vel: ViewMut<Vel>, mut res: UniqueViewMut<Resources>) {
    (&mut pos, &mut vel)
        .iter()
        .for_each(|(Pos(pos), Vel(speed))| {
            *pos += *speed;
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
                if res.rng.r#gen::<bool>() {
                    speed.y -= res.rng.range(0.0..6.0);
                }
            } else if pos.y < 0.0 {
                speed.y = 0.0;
                pos.y = 0.0;
            }
        });
}

fn render_bunnies(pos: View<Pos>, color: View<BunnyColor>, res: UniqueView<Resources>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    (&pos, &color)
        .iter()
        .for_each(|(Pos(pos), BunnyColor(color))| {
            draw.image(&res.sprite).position(*pos).color(*color);
        });

    draw.text(&format!("Bunnies: {}\nFPS: {:.2}", pos.len(), time::fps()))
        .size(10.0)
        .position(vec2(10.0, 10.0));

    gfx::render_to_frame(&draw).unwrap();
}

fn add_bunnies(
    mut entities: EntitiesViewMut,
    mut pos: ViewMut<Pos>,
    mut vel: ViewMut<Vel>,
    mut color: ViewMut<BunnyColor>,
    mut res: UniqueViewMut<Resources>,
) {
    entities.bulk_add_entity(
        (&mut pos, &mut vel, &mut color),
        (0..50).map(|_| {
            let speed = vec2(res.rng.range(0.0..10.0), res.rng.range(-5.0..5.0));
            let color = Color::rgb(res.rng.r#gen(), res.rng.r#gen(), res.rng.r#gen());
            (Pos(Vec2::ZERO), Vel(speed), BunnyColor(color))
        }),
    );
}

fn update(world: &mut World) {
    if is_mouse_btn_down(MouseButton::Left) {
        world.run(add_bunnies);
    }

    world.run(update_bunnies);
    world.run(render_bunnies);
}
