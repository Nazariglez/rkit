use rkit::{
    app::WindowConfig,
    draw::{self, RichTextLayout, TextStyle, TextStyles, create_draw_2d, text},
    gfx::{self, Color},
    math::vec2,
};

struct State {
    layout: RichTextLayout,
}

fn main() -> Result<(), String> {
    rkit::init_with(init)
        .with_window(WindowConfig::default().size(800, 300))
        .update(update)
        .run()
}

fn init() -> State {
    let font = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();
    let styles = TextStyles::new([
        ("under", TextStyle::new().underline(true).color(Color::AQUA)),
        (
            "strike",
            TextStyle::new().strikethrough(true).color(Color::YELLOW),
        ),
        (
            "plain",
            TextStyle::new().underline(false).strikethrough(false),
        ),
    ])
    .unwrap();
    let layout = text::rich_text(
        "[u]Underline spaces and wrapping[/u]\n[s]Strikethrough text[/s]\n[style:under]Named [style:plain]restored[/style] underline[/style]",
    )
    .font(&font)
    .styles(&styles)
    .size(28.0)
    .max_width(700.0)
    .layout()
    .unwrap();
    State { layout }
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw.rich_text(&state.layout)
        .position(vec2(40.0, 40.0))
        .shadow_offset(vec2(2.0, 2.0));
    gfx::render_to_frame(&draw).unwrap();
}
