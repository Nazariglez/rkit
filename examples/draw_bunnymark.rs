use rkit::app::set_window_title;
use rkit::draw::{draw_2d, Sprite};
use rkit::gfx::{self, Color};
use rkit::input::{is_mouse_btn_down, MouseButton};
use rkit::math::{vec2, Vec2};
use rkit::random::Rng;
use rkit::time;

struct Bunny {
    pos: Vec2,
    speed: Vec2,
    color: Color,
}

struct State {
    sprite: Sprite,
    rng: Rng,
    bunnies: Vec<Bunny>,
}

impl State {
    fn new() -> Result<Self, String> {
        let sprite = draw::create_sprite()
            .from_image(include_bytes!("assets/bunny.png"))
            .build()?;

        let rng = Rng::new();

        Ok(Self {
            sprite,
            rng,
            bunnies: vec![],
        })
    }

    fn spawn(&mut self, n: usize) {
        (0..n).for_each(|_| {
            self.bunnies.push(Bunny {
                pos: Vec2::ZERO,
                speed: vec2(self.rng.range(0.0..10.0), self.rng.range(-5.0..5.0)),
                color: Color::rgb(self.rng.gen(), self.rng.gen(), self.rng.gen()),
            })
        });
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(init).on_update(update).run()
}

fn init() -> State {
    let mut state = State::new().unwrap();
    state.spawn(1);
    state
}

fn update(state: &mut State) {
    set_window_title(&format!(
        "Bunnies: {} - FPS: {:.2}",
        state.bunnies.len(),
        time::fps()
    ));

    // add bunnies to our vector
    if is_mouse_btn_down(MouseButton::Left) {
        state.spawn(50);
    }

    // update positions
    let rng = &mut state.rng;
    state.bunnies.iter_mut().for_each(|b| {
        b.pos += b.speed;
        b.speed.y += 0.75;

        if b.pos.x > 800.0 {
            b.speed.x *= -1.0;
            b.pos.x = 800.0;
        } else if b.pos.x < 0.0 {
            b.speed.x *= -1.0;
            b.pos.x = 0.0
        }

        if b.pos.y > 600.0 {
            b.speed.y *= -0.85;
            b.pos.y = 600.0;
            if rng.gen::<bool>() {
                b.speed.y -= rng.range(0.0..6.0);
            }
        } else if b.pos.y < 0.0 {
            b.speed.y = 0.0;
            b.pos.y = 0.0;
        }
    });

    // draw
    let mut draw = draw_2d();
    draw.clear(Color::rgb(0.1, 0.2, 0.3));
    state.bunnies.iter().for_each(|b| {
        draw.image(&state.sprite).position(b.pos).color(b.color);
    });
    gfx::render_to_frame(&draw).unwrap();
}
