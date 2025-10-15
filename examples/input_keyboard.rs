use rkit::{draw::create_draw_2d, prelude::*, gfx::{self, Color}, input::{KeyCode, is_key_down, keys_released}, math::{Vec2, vec2}};

const MOVE_SPEED: f32 = 100.0;

#[derive(Component)]
struct Pos(Vec2);

#[derive(Component)]
struct LastKey(Option<KeyCode>);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_setup(setup_system)
        .on_update(update_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    cmds.spawn((Pos(vec2(400.0, 300.0)), LastKey(None)));
}

fn update_system(mut query: Query<(&mut Pos, &mut LastKey)>, time: Res<Time>) {
    if let Some(released_key) = keys_released().iter().last() {
        query.par_iter_mut().for_each(|(_, mut last_key)| {
            last_key.0 = Some(released_key);
        });
    }

    let movement = MOVE_SPEED * time.delta_f32();
    query.par_iter_mut().for_each(|(mut pos, _)| {
        if is_key_down(KeyCode::KeyW) {
            pos.0.y -= movement;
        }
        if is_key_down(KeyCode::KeyA) {
            pos.0.x -= movement;
        }
        if is_key_down(KeyCode::KeyS) {
            pos.0.y += movement;
        }
        if is_key_down(KeyCode::KeyD) {
            pos.0.x += movement;
        }
    });
}

fn draw_system(query: Query<(&Pos, &LastKey)>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    for (pos, last_key) in &query {
        draw.circle(50.0)
            .position(pos.0)
            .anchor(Vec2::splat(0.5))
            .color(Color::RED);

        draw.text("Use WASD to move the circle")
            .position(Vec2::splat(10.0));

        if let Some(key) = &last_key.0 {
            draw.text(&format!("Last key: {key:?}"))
                .position(vec2(10.0, 560.0));
        }
    }

    gfx::render_to_frame(&draw).unwrap();
}
