use rkit::app::window_size;
use rkit::draw::{self, Draw2D, Font, create_draw_2d};
use rkit::gfx::{self, Color};
use rkit::math::{Mat3, Rect, Vec2, vec2};

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
    rkit::init_with(State::new).update(update).run()
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let parent_translation = vec2(80.0, -40.0);
    let center = window_size() * 0.5 - parent_translation;
    draw.push_matrix(Mat3::from_translation(parent_translation));

    draw.text("Let's measure this text...")
        .font(&state.font)
        .translate(center)
        .size(40.0)
        .color(Color::ORANGE)
        .anchor(Vec2::splat(0.5))
        .scale(1.2);

    let bounds = draw.last_text_bounds();
    assert!((bounds.center() - center).length() < 0.01);

    let next_y = bounds.max().y + 40.0;
    draw.text("Positioned from the bounds")
        .font(&state.font)
        .translate(vec2(bounds.center().x, next_y))
        .size(24.0)
        .color(Color::AQUA)
        .anchor(vec2(0.5, 0.0));
    let next_bounds = draw.last_text_bounds();
    assert!((next_bounds.min().y - next_y).abs() < 0.01);

    draw_size(&mut draw, bounds);
    draw.pop_matrix();

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

    draw.text(&format!("{:.1}px", bounds.height()))
        .translate(vec2(bounds.max().x + 20.0, bounds.center().y))
        .anchor(vec2(0.0, 0.5));

    // show width
    draw.line(
        vec2(bounds.x(), bounds.max().y + 10.0),
        vec2(bounds.max().x, bounds.max().y + 10.0),
    )
    .width(2.0)
    .color(Color::GRAY);

    draw.text(&format!("{:.1}px", bounds.width()))
        .translate(vec2(bounds.center().x, bounds.max().y + 20.0))
        .anchor(vec2(0.5, 0.0));
}
