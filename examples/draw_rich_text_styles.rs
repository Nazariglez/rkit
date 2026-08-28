use rkit::{
    app::WindowConfig,
    draw::{
        self, RichTextLayout, TextDiagnosticCode, TextIcons, TextMarkupPolicy, TextSourceId,
        TextStyle, TextStyles, create_draw_2d, text,
    },
    gfx::{self, Color, TextureFilter},
    math::vec2,
};

struct State {
    layout: RichTextLayout,
}

fn main() -> Result<(), String> {
    rkit::init_with(init)
        .with_window(WindowConfig::default().size(900, 360))
        .update(update)
        .run()
}

fn init() -> State {
    let smooth = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();
    let pixel = draw::create_font(include_bytes!("./assets/kenney_pixel-webfont.ttf"))
        .with_nearest_filter(true)
        .build()
        .unwrap();
    let smooth_icon = draw::create_sprite()
        .from_image(include_bytes!("./assets/text_icon_smooth.png"))
        .build()
        .unwrap();
    let pixel_icon = draw::create_sprite()
        .from_image(include_bytes!("./assets/text_icon_pixel.png"))
        .with_filter(TextureFilter::Nearest)
        .build()
        .unwrap();
    let icons = TextIcons::new([("smooth", smooth_icon), ("pixel", pixel_icon)]).unwrap();
    let styles = TextStyles::new([
        (
            "title",
            TextStyle::new()
                .font(&smooth)
                .size(34.0)
                .line_height(42.0)
                .color(Color::YELLOW),
        ),
        (
            "pixel",
            TextStyle::new().font(&pixel).size(20.0).color(Color::AQUA),
        ),
        ("accent", TextStyle::new().size(25.0).color(Color::MAGENTA)),
    ])
    .unwrap();
    let invalid = text::rich_text("[style:missing]literal[/style]")
        .styles(&styles)
        .source_id(TextSourceId::new(7))
        .layout()
        .unwrap();
    assert_eq!(
        invalid.diagnostics()[0].code(),
        TextDiagnosticCode::UnknownStyleId
    );
    assert_eq!(invalid.diagnostics()[0].source_id(), TextSourceId::new(7));
    assert!(
        text::rich_text("value [ and [u]future[/u]")
            .styles(&styles)
            .markup_policy(TextMarkupPolicy::Strict)
            .layout()
            .unwrap()
            .diagnostics()
            .is_empty()
    );
    assert!(
        text::rich_text("[style:pixel][color:#fff]crossed[/style][/color]")
            .styles(&styles)
            .markup_policy(TextMarkupPolicy::Strict)
            .layout()
            .is_err()
    );
    assert!(
        text::rich_text("[style:missing]strict[/style]")
            .styles(&styles)
            .markup_policy(TextMarkupPolicy::Strict)
            .layout()
            .is_err()
    );

    let layout = text::rich_text(
        "[style:title]Named styles[/style]\nSmooth [icon:smooth] and [style:pixel]pixel [icon:pixel] text with [style:accent]nested size[/style] restored[/style].\nFallback: 日本語 שלום",
    )
    .styles(&styles)
    .icons(&icons)
    .font(&smooth)
    .size(18.0)
    .max_width(820.0)
    .layout()
    .unwrap();

    State { layout }
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.05, 0.06, 0.09));
    draw.rect(vec2(36.0, 36.0), state.layout.size())
        .stroke(1.0)
        .stroke_color(Color::GRAY);
    draw.rich_text(&state.layout)
        .position(vec2(36.0, 36.0))
        .shadow_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .shadow_offset(vec2(2.0, 2.0));
    gfx::render_to_frame(&draw).unwrap();
}
