use rkit::app::window_size;
use rkit::draw::{self, draw_2d, Font};
use rkit::gfx::{self, Color};
use rkit::math::vec2;

struct State {
    font: Font,
}

fn main() -> Result<(), String> {
    rkit::init_with(init).on_update(update).run()
}

fn init() -> State {
    let font = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();
    State { font }
}

fn update(s: &mut State) {
    let mut draw = draw_2d();
    draw.clear(Color::BLACK);

    let pos = window_size() * 0.5;
    let offset = vec2(0.0, 10.0);
    draw.text("Using Ubunut-B font.")
        .font(&s.font)
        .position(pos - offset)
        .color(Color::ORANGE)
        .size(48.0)
        .h_align_center()
        .v_align_bottom();

    draw.text("Using Arcade-Legacy (default) font.")
        .position(pos + offset)
        .color(Color::YELLOW)
        .size(10.0)
        .h_align_center()
        .v_align_top();

    gfx::render_to_frame(&draw).unwrap();
}