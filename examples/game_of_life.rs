use rkit::app::WindowConfig;
use rkit::draw::{self, draw_2d, Sprite};
use rkit::gfx::{self, Color};
use rkit::math::{uvec2, vec2};
use rkit::{random, time};

const WIDTH: usize = 100;
const HEIGHT: usize = 100;
const BYTES_LENGTH: usize = WIDTH * HEIGHT * 4;

struct State {
    sprite: Sprite,
    current_bytes: [u8; BYTES_LENGTH],
    previous_bytes: [u8; BYTES_LENGTH],
    count: f32,
    dirty: bool,
}

impl State {
    fn new() -> Self {
        let current_bytes = [255; BYTES_LENGTH];
        let previous_bytes = current_bytes;

        let sprite = draw::create_sprite()
            .from_bytes(&current_bytes, WIDTH as _, HEIGHT as _)
            .with_write_flag(true)
            .build()
            .unwrap();

        let mut state = State {
            sprite,
            current_bytes,
            previous_bytes,
            count: 0.0,
            dirty: false,
        };

        // randomize
        for _ in 0..500 {
            let x = random::range(0..WIDTH);
            let y = random::range(0..HEIGHT);

            let neighbors = get_neighbors(x as _, y as _);
            neighbors.iter().for_each(|(x, y)| {
                let valid_coords = index(*x, *y).is_some();
                if valid_coords {
                    state.set_color(Color::RED, *x as _, *y as _);
                }
            });
        }
        state.swap_data();

        state
    }

    fn is_alive(&self, x: usize, y: usize) -> bool {
        let neighbors = get_neighbors(x as _, y as _);
        let count = neighbors.iter().fold(0, |sum, (x, y)| {
            let idx = index(*x, *y);
            match idx {
                Some(idx) => {
                    let is_red =
                        is_red_color(&self.previous_bytes[idx..idx + 4].try_into().unwrap());
                    if is_red {
                        sum + 1
                    } else {
                        sum
                    }
                }
                _ => sum,
            }
        });

        let was_alive = match index(x as _, y as _) {
            Some(idx) => is_red_color(&self.previous_bytes[idx..idx + 4].try_into().unwrap()),
            _ => false,
        };

        if was_alive {
            count == 2 || count == 3
        } else {
            count == 3
        }
    }

    fn swap_data(&mut self) {
        std::mem::swap(&mut self.current_bytes, &mut self.previous_bytes);
        self.dirty = true;
    }

    fn set_color(&mut self, color: Color, x: usize, y: usize) {
        if let Some(idx) = index(x as _, y as _) {
            self.current_bytes[idx..idx + 4].copy_from_slice(&color.to_rgba_u8());
        }
    }
}

fn main() -> Result<(), String> {
    let win = WindowConfig {
        size: uvec2((WIDTH * 4) as _, (HEIGHT * 4) as _),
        ..Default::default()
    };
    rkit::init_with(State::new)
        .with_window(win)
        .on_update(update)
        .run()
}

fn update(state: &mut State) {
    state.count += time::delta_f32();

    // update each 300ms
    if state.count >= 0.3 {
        state.count = 0.0;

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let color = if state.is_alive(x, y) {
                    Color::RED
                } else {
                    Color::WHITE
                };

                state.set_color(color, x, y);
            }
        }

        state.swap_data();
    }

    // draw
    if state.dirty {
        gfx::write_texture(state.sprite.texture())
            .from_data(&state.current_bytes)
            .build()
            .unwrap();

        state.dirty = false;
    }

    let mut draw = draw_2d();
    draw.clear(Color::BLACK);
    draw.image(&state.sprite); // TODO scale x 4
    gfx::render_to_frame(&draw).unwrap();
}

fn index(x: isize, y: isize) -> Option<usize> {
    if x < 0 || y < 0 {
        return None;
    }

    let x = x as usize;
    let y = y as usize;
    let index = ((y * WIDTH) + x) * 4;
    if index >= BYTES_LENGTH {
        None
    } else {
        Some(index)
    }
}

#[inline]
fn is_red_color(bytes: &[u8; 4]) -> bool {
    bytes == &[255, 0, 0, 255]
}

#[rustfmt::skip]
fn get_neighbors(ix: isize, iy: isize) -> [(isize, isize); 8] {
    [
        (ix - 1, iy - 1), (ix, iy - 1), (ix + 1, iy - 1),
        (ix - 1, iy),                   (ix + 1, iy),
        (ix - 1, iy + 1), (ix, iy + 1), (ix + 1, iy + 1),
    ]
}
