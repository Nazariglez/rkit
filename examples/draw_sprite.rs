use rkit::app::window_size;
use rkit::draw::{draw_2d, Sprite};
use rkit::gfx::{self, Color};
use rkit::math::Vec2;

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
    for i in 0..100 {
        draw.image(&s.sprite).position(Vec2::splat(i as f32 * 10.0));
    }
    gfx::render_to_frame(&draw).unwrap();
}
