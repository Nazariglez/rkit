use rkit::app::window_size;
use rkit::draw::{draw_2d, Sprite};
use rkit::gfx::{self, Color};
use rkit::math::vec2;

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
    println!("-- START --");
    let pos = window_size() * 0.5 - s.sprite.size() * 0.5;

    let mut draw = draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));
    draw.triangle(vec2(400.0, 100.0), vec2(100.0, 500.0), vec2(700.0, 500.0));
    draw.image(&s.sprite).position(pos);
    draw.image(&s.sprite).position(pos + vec2(100.0, 100.0));
    draw.triangle(vec2(450.0, 150.0), vec2(150.0, 550.0), vec2(750.0, 550.0))
        .color(Color::BLUE.with_alpha(0.1));
    gfx::render_to_frame(&draw).unwrap();
    println!("-- END --");
}
