use draw::create_draw_2d;
use rkit::ecs::prelude::*;
use rkit::ecs::{App, OnCleanup, OnRender, OnSetup, OnUpdate};
use rkit::gfx::Color;
use rkit::math::{vec2, Vec2};
use rkit::{gfx, time};

fn main() -> Result<(), String> {
    App::new()
        .add_systems(OnSetup, setup_system)
        .add_systems(OnUpdate, update_system)
        .add_systems(OnRender, draw_system)
        .run()
}

#[derive(Component)]
struct Pos(Vec2);

#[derive(Component)]
struct Rect(Vec2);

fn setup_system(mut cmds: Commands) {
    println!("Hello World!");
    cmds.spawn((Pos(vec2(400.0, 300.0)), Rect(Vec2::splat(200.0))));
}

fn update_system(mut query: Query<&mut Rect>) {
    let elapsed = time::elapsed_f32() * 2.0;
    let anim = vec2(elapsed.sin(), elapsed.cos());
    query.par_iter_mut().for_each(|mut rect| {
        rect.0 = Vec2::splat(200.0) + anim * 50.0;
    });
}

fn draw_system(query: Query<(&Pos, &Rect)>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::ORANGE);
    for (pos, rect) in &query {
        draw.rect(Vec2::ZERO, rect.0)
            .anchor(Vec2::splat(0.5))
            .translate(pos.0);
    }

    gfx::render_to_frame(&draw).unwrap();
}
