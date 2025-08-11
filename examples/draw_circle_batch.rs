use rkit::{
    draw::batch::CircleBatcher,
    gfx::{self, Color, Renderer},
    math::{Mat4, Vec2, vec2},
    prelude::*,
};

#[derive(Resource, Deref)]
struct Batcher(CircleBatcher);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_systems(OnSetup, setup)
        .add_systems(OnUpdate, update)
        .run()
}

impl Batcher {
    fn new() -> Self {
        let mut batcher = CircleBatcher::new().unwrap();
        batcher.set_projection(Mat4::orthographic_rh(0.0, 800.0, 600.0, 0.0, 0.0, 1.0));
        Batcher(batcher)
    }
}

fn setup(mut cmds: Commands) {
    cmds.insert_resource(Batcher::new());
}

fn update(mut batcher: ResMut<Batcher>, time: Res<Time>) {
    // clear last frame batch
    batcher.clear();

    // -- push new circles

    // just a filled circle with a gradient
    batcher
        .fill(vec2(400.0, 300.0), 50.0)
        .inner_color(Color::rgb(0.1, 0.2, 0.7))
        .outer_color(Color::BLUE);

    // stroke the circle with a white line (default color)
    batcher.stroke(vec2(400.0, 300.0), 50.0);

    // simple arc from-to angle
    batcher
        .arc(Vec2::splat(100.0), 30.0)
        .width(10.0)
        .start_angle(90f32.to_radians())
        .end_angle(180f32.to_radians())
        .color(Color::GREEN);

    // this one is special, a circular load bar with a gradient
    // from tail to head that converts to solid color at the end
    batcher
        .load_bar(vec2(700.0, 500.0), 70.0)
        .width(20.0)
        .inner_color(Color::MAGENTA)
        .outer_color(Color::ORANGE)
        .progress((time.elapsed_f32() % 10.0) / 10.0);

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
}
