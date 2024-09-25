use rkit::app::window_size;
use rkit::draw::{create_draw_2d, Sprite};
use rkit::gfx::{self, Color};
use rkit::math::{vec2, Vec2};
use rkit::time;
use std::ops::Rem;

struct State {
    sprite: Sprite,
}

impl State {
    fn new() -> Result<Self, String> {
        let sprite = draw::create_sprite()
            .from_image(include_bytes!("assets/ferris.png"))
            .build()?;

        Ok(Self { sprite })
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(|| State::new().unwrap())
        .on_update(update)
        .run()
}

fn update(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    let t = time::elapsed_f32();
    let dt = time::delta_f32();
    draw.image(&s.sprite) // draw the sprite
        .translate(window_size() * 0.5) // change position
        .anchor(Vec2::splat(0.5)) // set position anchor to center
        .pivot(Vec2::splat(0.5)) // set scale/rotation pivot point to center
        .flip_x(true) // flip the image horizontally
        .skew(vec2(t.sin(), t.cos())) // skew the image
        .scale(Vec2::splat(1.5 + t.sin() * 0.3)) // scale the image
        .rotation(t.rem(360.0).to_radians() * 10.0 * dt); // rotate the image from the pivot point

    gfx::render_to_frame(&draw).unwrap();
}
