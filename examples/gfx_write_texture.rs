use draw::create_draw_2d;
use rkit::app::window_size;
use rkit::draw::{self, Sprite};
use rkit::gfx::{self, Color};

const COLORS: [Color; 6] = [
    Color::WHITE,
    Color::RED,
    Color::GREEN,
    Color::BLUE,
    Color::ORANGE,
    Color::rgb(0.1, 0.2, 0.3),
];

struct State {
    // sprite includes a sampler, it's more convenient use it
    // but the process is the same with a raw Texture
    sprite: Sprite,
    bytes: Vec<u8>,
    count: usize,
    color: Color,
    step: usize,
}

impl State {
    fn new() -> Self {
        let size = window_size().as_uvec2();
        let channels = 4;
        let len = size.element_product() * channels;
        let bytes = vec![0; len as _];

        let texture = draw::create_sprite()
            .from_bytes(&bytes, size.x, size.y)
            .with_write_flag(true)
            .build()
            .unwrap();

        Self {
            sprite: texture,
            bytes,
            count: 0,
            color: Color::WHITE,
            step: 0,
        }
    }
}

fn main() {
    rkit::init_with(State::new).update(update).run().unwrap()
}

fn update(state: &mut State) {
    // update the data that will be send to the gpu
    update_bytes(state);

    // Update the texture with the new data
    gfx::write_texture(state.sprite.texture())
        .from_data(&state.bytes)
        .build()
        .unwrap();

    // Draw the texture using the draw 2d API for convenience
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw.image(&state.sprite);
    gfx::render_to_frame(&draw);
}

fn update_bytes(state: &mut State) {
    for _ in 0..100 {
        let index = state.count * 4;
        state.bytes[index..index + 4].copy_from_slice(&state.color.to_rgba_u8());
        state.count += 9;

        let len = state.bytes.len() / 4;
        if state.count >= len {
            state.count -= len;
            state.step = (state.step + 1) % COLORS.len();
            state.color = COLORS[state.step];
        }
    }
}
