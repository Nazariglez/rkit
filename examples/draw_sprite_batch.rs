use rkit::{
    draw::{Sprite, batch::SpriteBatcher, create_draw_2d, create_sprite},
    gfx::{self, Color, Renderer},
    math::{Mat4, Vec2, vec2},
    prelude::*,
    random::{self, Rng},
};

#[derive(Resource, Deref)]
struct Batcher(SpriteBatcher);

struct Bunny {
    pos: Vec2,
    speed: Vec2,
    color: Color,
}

#[derive(Resource)]
struct AppState {
    sprite: Sprite,
    bunnies: Vec<Bunny>,
}

impl AppState {
    fn spawn(&mut self, n: usize) {
        (0..n).for_each(|_| {
            self.bunnies.push(Bunny {
                pos: Vec2::ZERO,
                speed: vec2(random::range(0.0..10.0), random::range(-5.0..5.0)),
                color: Color::rgb(random::r#gen(), random::r#gen(), random::r#gen()),
            })
        });
    }
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_systems(OnSetup, on_setup)
        .add_systems(OnUpdate, on_update)
        .add_systems(OnRender, on_render)
        .run()
}

fn on_setup(mut cmds: Commands) {
    let mut batcher = SpriteBatcher::new().unwrap();
    batcher.set_transform(Mat4::orthographic_rh(0.0, 800.0, 600.0, 0.0, 0.0, 1.0));

    cmds.insert_resource(Batcher(batcher));

    let sprite = create_sprite()
        .from_image(include_bytes!("assets/bunny.png"))
        .build()
        .unwrap();

    cmds.insert_resource(AppState {
        sprite,
        bunnies: vec![],
    });
}

fn on_update(mut state: ResMut<AppState>, mouse: Res<Mouse>) {
    if mouse.is_down(MouseButton::Left) {
        state.spawn(500);
    }

    // update positions
    state.bunnies.iter_mut().for_each(|b| {
        b.pos += b.speed;
        b.speed.y += 0.75;

        if b.pos.x > 800.0 {
            b.speed.x *= -1.0;
            b.pos.x = 800.0;
        } else if b.pos.x < 0.0 {
            b.speed.x *= -1.0;
            b.pos.x = 0.0
        }

        if b.pos.y > 600.0 {
            b.speed.y *= -0.85;
            b.pos.y = 600.0;
            if random::r#gen::<bool>() {
                b.speed.y -= random::range(0.0..6.0);
            }
        } else if b.pos.y < 0.0 {
            b.speed.y = 0.0;
            b.pos.y = 0.0;
        }
    });
}

fn on_render(state: Res<AppState>, mut batcher: ResMut<Batcher>, time: Res<Time>) {
    // clear last frame batch
    batcher.clear();

    state.bunnies.iter().for_each(|bunny| {
        batcher.sprite(&state.sprite, bunny.pos).color(bunny.color);
    });

    // upload buffers to the gpu if needed
    batcher.upload().unwrap();

    // base renderer, we use it to clear the screen
    let mut renderer = Renderer::new();
    renderer
        .begin_pass()
        .clear_color(Color::rgb(0.1, 0.2, 0.3).as_linear());

    // apply our bartch to the renderer we just created
    batcher.apply_pass_to(&mut renderer);

    // and finally we render to the frame
    gfx::render_to_frame(&renderer).unwrap();

    // ui
    let mut draw = create_draw_2d();

    draw.text(&format!(
        "Bunnies: {}\nFPS: {:.2}",
        state.bunnies.len(),
        time.fps(),
    ))
    .size(10.0)
    .position(vec2(10.0, 10.0));

    gfx::render_to_frame(&draw).unwrap();
}
