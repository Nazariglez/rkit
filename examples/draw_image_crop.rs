use rkit::draw::{Sprite, create_draw_2d};
use rkit::gfx::{self, Color};
use rkit::math::{Vec2, vec2};

struct State {
    img: Sprite,
}

impl State {
    fn new() -> Result<Self, String> {
        let sprite = draw::create_sprite()
            .from_image(include_bytes!("assets/rust-logo-512x512.png"))
            .build()?;

        Ok(Self { img: sprite })
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
}

fn update(state: &mut State) {
    let Vec2 { x: ww, y: hh } = state.img.size();

    let mut draw = create_draw_2d();
    draw.clear(Color::WHITE);

    // Right side of the logo
    draw.image(&state.img)
        .position(vec2(100.0, 50.0))
        .size(vec2(ww * 0.5, hh))
        .crop(vec2(ww * 0.5, 0.0), vec2(ww * 0.5, hh));

    // Left side of the logo
    draw.image(&state.img)
        .position(vec2(450.0, 50.0))
        .size(vec2(ww * 0.5, hh))
        .crop(vec2(0.0, 0.0), vec2(ww * 0.5, hh));

    gfx::render_to_frame(&draw).unwrap();
}
