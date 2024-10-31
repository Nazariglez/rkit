use rkit::app::window_size;
use rkit::draw::{create_draw_2d, Sprite};
use rkit::filters::{PixelateFilter, PostProcess};
use rkit::gfx::{self, Color};

struct State {
    sprite: Sprite,
    pixelate: PixelateFilter,
}

impl State {
    fn new() -> Result<Self, String> {
        let sprite = draw::create_sprite()
            .from_image(include_bytes!("assets/ferris.png"))
            .build()?;

        let pixelate = PixelateFilter::new()?;

        Ok(Self { sprite, pixelate })
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
}

fn update(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    draw.image(&s.sprite)
        .position(window_size() * 0.5 - s.sprite.size() * 0.5);

    gfx::render_to_frame(&PostProcess {
        filters: &[&s.pixelate],
        // filters: &[],
        render: &draw,
        pixelated: true,
    })
    .unwrap();
}
