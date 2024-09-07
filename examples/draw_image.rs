use rkit::app::window_size;
use rkit::draw::draw_2d;
use rkit::gfx::{self, Color, Texture};
use rkit::math::vec2;

struct State {
    texture: Texture,
}

impl State {
    fn new() -> Result<Self, String> {
        let texture = gfx::create_texture()
            .from_image(include_bytes!("assets/ferris.png"))
            .build()?;

        Ok(Self { texture })
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
    draw.image(&s.texture).position(window_size() * 0.5);
    gfx::render_to_frame(&draw).unwrap();
}
