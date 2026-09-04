use rkit::app::WindowConfig;
use rkit::draw::{self, Sprite, create_draw_2d};
use rkit::gfx::{self, Color};
use rkit::math::vec2;
use rkit::time;

const WIDTH: u32 = 301;
const HEIGHT: u32 = 197;
const REGENERATION_INTERVAL: f32 = 2.0;

struct State {
    sprite: Sprite,
    bytes: Vec<u8>,
    elapsed: f32,
    regeneration_elapsed: f32,
}

impl State {
    fn new() -> Self {
        let mut bytes = vec![0; (WIDTH * HEIGHT * 4) as usize];
        fill_pixels(&mut bytes, 0.0);
        let sprite = draw::create_sprite()
            .from_bytes(&bytes, WIDTH, HEIGHT)
            .with_write_flag(true)
            .with_mipmaps()
            .build()
            .unwrap();

        Self {
            sprite,
            bytes,
            elapsed: 0.0,
            regeneration_elapsed: 0.0,
        }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new)
        .with_window(
            WindowConfig::default()
                .title("Writable texture mipmaps")
                .size(640, 300)
                .resizable(false),
        )
        .update(update)
        .run()
}

fn update(state: &mut State) {
    let delta = time::delta_f32();
    state.elapsed += delta;
    state.regeneration_elapsed += delta;

    fill_pixels(&mut state.bytes, state.elapsed);
    gfx::write_texture(state.sprite.texture())
        .from_data(&state.bytes)
        .build()
        .unwrap();

    if state.regeneration_elapsed >= REGENERATION_INTERVAL {
        gfx::generate_mipmaps(state.sprite.texture()).unwrap();
        state.regeneration_elapsed = 0.0;
    }

    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.035, 0.04, 0.055));
    draw.text("Level 0: every frame")
        .translate(vec2(24.0, 20.0))
        .size(8.0)
        .color(Color::WHITE);
    draw.image(&state.sprite)
        .position(vec2(24.0, 55.0))
        .size(vec2(WIDTH as f32, HEIGHT as f32));

    draw.text("Mipmaps: every 2 seconds")
        .translate(vec2(360.0, 20.0))
        .size(8.0)
        .color(Color::WHITE);
    draw.image(&state.sprite)
        .position(vec2(450.0, 100.0))
        .size(vec2(75.0, 49.0));

    gfx::render_to_frame(&draw).unwrap();
}

fn fill_pixels(bytes: &mut [u8], elapsed: f32) {
    let stripe = (((elapsed * 90.0) as u32) % (WIDTH + 80)) as i32 - 40;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let checker = (x / 16 + y / 16) % 2 == 0;
            let moving = (x as i32 - stripe).abs() < 38;
            let rgba = if moving {
                if checker {
                    [255, 92, 40, 255]
                } else {
                    [255, 196, 32, 255]
                }
            } else if checker {
                [34, 48, 78, 255]
            } else {
                [12, 18, 34, 255]
            };
            let index = ((y * WIDTH + x) * 4) as usize;
            bytes[index..index + 4].copy_from_slice(&rgba);
        }
    }
}
