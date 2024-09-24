use draw::Transform2D;
use etagere::euclid::Trig;
use rkit::app::window_size;
use rkit::draw::{draw_2d, Sprite};
use rkit::gfx::{self, Color};
use rkit::math::{bvec2, vec2, Vec2};
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
    let mut draw = draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    let t = time::elapsed_f32();
    draw.push_matrix(
        Transform2D::new()
            .set_position(window_size() * 0.5)
            .set_size(s.sprite.size())
            .set_anchor(Vec2::splat(0.5))
            .set_pivot(Vec2::splat(0.5))
            .set_flip(bvec2(true, false))
            .set_skew(vec2(t.sin(), t.cos()))
            .set_scale(Vec2::splat(1.5 + t.sin() * 0.3))
            .set_rotation(t.rem(360.0).to_radians() * 10.0)
            .as_mat3(),
    );

    draw.image(&s.sprite); //.position(pos);

    draw.pop_matrix();

    gfx::render_to_frame(&draw).unwrap();
}
