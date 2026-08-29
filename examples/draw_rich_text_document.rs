use rkit::{
    app::WindowConfig,
    draw::{
        self, Font, RichTextDocument, RichTextIcon, RichTextLayout, TextIconAlign, TextIcons,
        TextSourceId, TextStyle, TextStyles, create_draw_2d, text,
    },
    gfx::{AsRenderer, Color, TextureFilter},
    math::vec2,
};

struct State {
    font: Font,
    literal: RichTextLayout,
    rebuilt: RichTextLayout,
    markup: RichTextLayout,
}

fn main() -> Result<(), String> {
    rkit::init_with(init)
        .with_window(WindowConfig::default().size(980, 360))
        .update(update)
        .run()
}

fn init() -> State {
    let font = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();
    let icon = draw::create_sprite()
        .from_image(include_bytes!("./assets/text_icon_pixel.png"))
        .with_filter(TextureFilter::Nearest)
        .build()
        .unwrap();
    let icons = TextIcons::new([("confirm", icon)]).unwrap();
    let styles = TextStyles::new([
        (
            "title",
            TextStyle::new()
                .size(26.0)
                .color(Color::rgb(0.45, 0.94, 0.97)),
        ),
        ("key", TextStyle::new().color(Color::rgb(1.0, 0.75, 0.35))),
    ])
    .unwrap();

    let player_text = String::from(
        "Literal: [icon:missing] [style:title]name[/style]\n[color:#ff0000] [[ \u{FFFC} \u{200B}",
    );
    let mut document = RichTextDocument::new();
    document.source(TextSourceId::new(7), |document| {
        document.text(&player_text);
        document.text("\nPress ");
        document.style("key", |document| {
            document.text("Space [still literal]");
        });
        document.text(" ");
        document.icon(
            RichTextIcon::new("confirm")
                .size(28.0)
                .align(TextIconAlign::Baseline),
        );
    });
    drop(player_text);
    let literal = text::rich_document(&document)
        .icons(&icons)
        .styles(&styles)
        .font(&font)
        .size(18.0)
        .max_width(860.0)
        .layout()
        .unwrap();
    assert!(literal.diagnostics().is_empty());

    document.clear();
    assert!(document.is_empty());
    document.text("default ");
    document.icon(RichTextIcon::new("confirm").size(24.0));
    document.text("  middle ");
    document.icon(
        RichTextIcon::new("confirm")
            .size(24.0)
            .align(TextIconAlign::Middle),
    );
    document.text("  baseline ");
    document.icon(
        RichTextIcon::new("confirm")
            .size(24.0)
            .align(TextIconAlign::Baseline),
    );
    document.text("  top ");
    document.icon(
        RichTextIcon::new("confirm")
            .size(24.0)
            .align(TextIconAlign::Top),
    );
    document.text("  bottom ");
    document.icon(
        RichTextIcon::new("confirm")
            .size(24.0)
            .align(TextIconAlign::Bottom),
    );
    let rebuilt = text::rich_document(&document)
        .icons(&icons)
        .styles(&styles)
        .font(&font)
        .size(18.0)
        .layout()
        .unwrap();

    let markup = text::rich_text(
        "Markup: [style:title]styled[/style] [icon:confirm size=28 align=baseline]",
    )
    .icons(&icons)
    .styles(&styles)
    .font(&font)
    .size(18.0)
    .layout()
    .unwrap();

    let mut invalid = RichTextDocument::new();
    invalid.icon("missing");
    assert!(text::rich_document(&invalid).layout().is_err());
    drop((invalid, document, styles, icons));

    State {
        font,
        literal,
        rebuilt,
        markup,
    }
}

fn update(state: &mut State) {
    const LEFT: f32 = 24.0;
    const HEADING_GAP: f32 = 28.0;
    const SECTION_GAP: f32 = 24.0;

    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.06, 0.07, 0.1));

    let mut y = 20.0;
    draw.text("Old snapshot: literal content survives clear and drop")
        .font(&state.font)
        .size(18.0)
        .position(vec2(LEFT, y));
    y += HEADING_GAP;
    draw.rich_text(&state.literal).position(vec2(LEFT, y));
    y += state.literal.size().y + SECTION_GAP;

    draw.text("Same authored syntax through markup")
        .font(&state.font)
        .size(18.0)
        .position(vec2(LEFT, y));
    y += HEADING_GAP;
    draw.rich_text(&state.markup).position(vec2(LEFT, y));
    y += state.markup.size().y + SECTION_GAP;

    draw.text("New snapshot: rebuilt document")
        .font(&state.font)
        .size(18.0)
        .position(vec2(LEFT, y));
    y += HEADING_GAP;
    draw.rich_text(&state.rebuilt).position(vec2(LEFT, y));

    draw.render(None).unwrap();
}
