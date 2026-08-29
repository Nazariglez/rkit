use rkit::{
    app::WindowConfig,
    draw::{self, RichTextLayout, create_draw_2d, text},
    gfx::{self, Color},
    math::vec2,
    time,
};

struct State {
    layout: RichTextLayout,
}

fn main() -> Result<(), String> {
    rkit::init_with(init)
        .with_window(WindowConfig::default().size(800, 260))
        .update(update)
        .run()
}

fn init() -> State {
    let font = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();
    let layout = text::rich_text("Reveal ffi, e\u{301}, 👨‍👩‍👧‍👦, 日本語 and שלום")
        .font(&font)
        .size(30.0)
        .max_width(720.0)
        .layout()
        .unwrap();
    assert!(layout.reveal_units() > 0);
    let _ = layout.hit_test(vec2(1.0, 1.0));
    State { layout }
}

fn update(state: &mut State) {
    let units = state.layout.reveal_units();
    let reveal = ((time::elapsed_f32() * 8.0) as usize) % (units + 1);
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw.rich_text(&state.layout)
        .position(vec2(40.0, 60.0))
        .reveal(reveal);
    gfx::render_to_frame(&draw).unwrap();
}
