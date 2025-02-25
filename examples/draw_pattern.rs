use rkit::app::window_size;
use rkit::draw::{Sprite, create_draw_2d};
use rkit::gfx::{self, Color};
use rkit::math::vec2;
use rkit::time;

struct State {
    img: Sprite,
}

impl State {
    fn new() -> Result<Self, String> {
        let img = draw::create_sprite()
            .from_image(include_bytes!("assets/pattern.png"))
            .build()?;

        Ok(Self { img })
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let t = time::elapsed_f32();
    draw.pattern(&state.img)
        .size(window_size())
        .image_offset(vec2(t.sin(), t.cos()) * vec2(100.0, 20.0));

    gfx::render_to_frame(&draw).unwrap();
}
