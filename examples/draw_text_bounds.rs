use rkit::app::window_size;
use rkit::draw::{self, create_draw_2d, Draw2D, Font};
use rkit::gfx::{self, Color};
use rkit::math::{vec2, Rect};

struct State {
    font: Font,
}

impl State {
    fn new() -> Self {
        let font = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
            .build()
            .unwrap();

        Self { font }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).on_update(update).run()
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    draw.text("Let's measure this text...")
        .font(&state.font)
        .position(window_size() * 0.5)
        .size(40.0)
        .color(Color::ORANGE)
        .h_align_center()
        .v_align_middle();

    // get text bounds
    let bounds = draw.last_text_bounds();

    // draw the size
    draw_size(&mut draw, bounds);

    gfx::render_to_frame(&draw).unwrap();
}

fn draw_size(draw: &mut Draw2D, bounds: Rect) {
    // show height
    draw.line(
        vec2(bounds.max().x + 10.0, bounds.y()),
        vec2(bounds.max().x + 10.0, bounds.max().y),
    )
    .width(2.0)
    .color(Color::GRAY);

    draw.text(&format!("{}px", bounds.height()))
        .position(vec2(bounds.max().x + 20.0, bounds.center().y))
        .v_align_middle();

    // show width
    draw.line(
        vec2(bounds.x(), bounds.max().y + 10.0),
        vec2(bounds.max().x, bounds.max().y + 10.0),
    )
    .width(2.0)
    .color(Color::GRAY);

    draw.text(&format!("{:.2}px", bounds.width()))
        .position(vec2(bounds.center().x, bounds.max().y + 20.0))
        .h_align_center();
}
