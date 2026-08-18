use rkit::{
    app::{WindowConfig, window_size},
    draw::{self, Font, RichTextBuilder, RichTextLayout, TextIcons, create_draw_2d, text},
    gfx::{self, Color, TextureFilter},
    math::{Vec2, vec2},
};

struct State {
    font: Font,
    icons: TextIcons,
    default_icon: RichTextLayout,
    tall_icon: RichTextLayout,
    wrapped: RichTextLayout,
    aligned_left: RichTextLayout,
    aligned_center: RichTextLayout,
    aligned_right: RichTextLayout,
    colors: RichTextLayout,
    banner_and_fallback: RichTextLayout,
    unicode: RichTextLayout,
    transformed: RichTextLayout,
}

fn main() -> Result<(), String> {
    rkit::init_with(init)
        .with_window(WindowConfig::default().size(1100, 800))
        .update(update)
        .run()
}

fn init() -> State {
    let font = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();
    let icons = TextIcons::new([
        (
            "pixel",
            text::TextIcon::from_image(include_bytes!("./assets/text_icon_pixel.png"))
                .unwrap()
                .sampling(TextureFilter::Nearest),
        ),
        (
            "smooth",
            text::TextIcon::from_image(include_bytes!("./assets/text_icon_smooth.png")).unwrap(),
        ),
        (
            "banner",
            text::TextIcon::from_image(include_bytes!("./assets/text_icon_banner.png")).unwrap(),
        ),
    ])
    .unwrap();

    let default_icon = rich_layout(
        "Default 1em [icon:pixel] and smooth [icon:smooth] icons.",
        &icons,
        &font,
    )
    .layout()
    .unwrap();
    let tall_icon = rich_layout(
        "Normal line\nTall [icon:smooth size=42] middle line\nNormal line",
        &icons,
        &font,
    )
    .layout()
    .unwrap();
    let wrapped = rich_layout(
        "A banner [icon:banner] changes wrapping before this final phrase.",
        &icons,
        &font,
    )
    .max_width(260.0)
    .layout()
    .unwrap();
    let alignment = "[icon:pixel] short icon line\nA longer reference line";
    let aligned_left = rich_layout(alignment, &icons, &font)
        .max_width(230.0)
        .h_align_left()
        .layout()
        .unwrap();
    let aligned_center = rich_layout(alignment, &icons, &font)
        .max_width(230.0)
        .h_align_center()
        .layout()
        .unwrap();
    let aligned_right = rich_layout(alignment, &icons, &font)
        .max_width(230.0)
        .h_align_right()
        .layout()
        .unwrap();
    let colors = rich_layout(
        "Tint [color:#73eff7][icon:smooth] cyan[/color]; \
         [color:#ff0000]red [color:#0000ff][icon:pixel][/color] red[/color].",
        &icons,
        &font,
    )
    .max_width(430.0)
    .layout()
    .unwrap();
    let banner_and_fallback = rich_layout(
        "2:1 [icon:banner size=18] banner. [icon:missing] [icon:pixel size=0] [icon:]",
        &icons,
        &font,
    )
    .layout()
    .unwrap();
    let unicode = rich_layout(
        "CJK: 漢字 [icon:smooth] 日本語 | RTL: שלום [icon:pixel] 42 text",
        &icons,
        &font,
    )
    .max_width(540.0)
    .layout()
    .unwrap();
    let transformed = rich_layout("Origin + rotation [icon:pixel]", &icons, &font)
        .layout()
        .unwrap();

    State {
        font,
        icons,
        default_icon,
        tall_icon,
        wrapped,
        aligned_left,
        aligned_center,
        aligned_right,
        colors,
        banner_and_fallback,
        unicode,
        transformed,
    }
}

fn rich_layout<'a>(content: &'a str, icons: &'a TextIcons, font: &'a Font) -> RichTextBuilder<'a> {
    text::rich_text(content).icons(icons).font(font).size(18.0)
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.06, 0.07, 0.1));

    label(
        &mut draw,
        "1em + nearest and linear sampling",
        vec2(28.0, 20.0),
    );
    draw.rich_text(&state.default_icon)
        .position(vec2(28.0, 40.0));

    let tall_pos = vec2(28.0, 96.0);
    label(&mut draw, "Tall icon changes one line", vec2(28.0, 75.0));
    draw.rect(tall_pos, state.tall_icon.size())
        .stroke_color(Color::MAGENTA)
        .stroke(1.0);
    draw.rich_text(&state.tall_icon).position(tall_pos);

    let wrap_pos = vec2(28.0, 225.0);
    label(
        &mut draw,
        "Icon-caused wrap and exact layout bounds",
        wrap_pos,
    );
    let wrapped_pos = wrap_pos + vec2(0.0, 20.0);
    draw.rect(wrapped_pos, state.wrapped.size())
        .stroke_color(Color::MAGENTA)
        .stroke(1.0);
    draw.rich_text(&state.wrapped).position(wrapped_pos);

    label(
        &mut draw,
        "Alignment: left / center / right",
        vec2(600.0, 20.0),
    );
    let left_pos = vec2(600.0, 42.0);
    let center_pos = left_pos + vec2(0.0, state.aligned_left.size().y + 16.0);
    let right_pos = center_pos + vec2(0.0, state.aligned_center.size().y + 16.0);
    draw.rect(left_pos, state.aligned_left.size())
        .stroke_color(Color::GRAY)
        .stroke(1.0);
    draw.rect(center_pos, state.aligned_center.size())
        .stroke_color(Color::GRAY)
        .stroke(1.0);
    draw.rect(right_pos, state.aligned_right.size())
        .stroke_color(Color::GRAY)
        .stroke(1.0);
    draw.rich_text(&state.aligned_left).position(left_pos);
    draw.rich_text(&state.aligned_center).position(center_pos);
    draw.rich_text(&state.aligned_right).position(right_pos);

    let color_label = right_pos + vec2(0.0, state.aligned_right.size().y + 24.0);
    label(&mut draw, "Tint and nested color restore", color_label);
    draw.rich_text(&state.colors)
        .position(color_label + vec2(0.0, 21.0));

    let fallback_label = wrapped_pos + vec2(0.0, state.wrapped.size().y + 28.0);
    label(
        &mut draw,
        "Aspect ratio and literal fallback",
        fallback_label,
    );
    let fallback_pos = fallback_label + vec2(0.0, 21.0);
    draw.rich_text(&state.banner_and_fallback)
        .position(fallback_pos);

    let unicode_label = fallback_pos + vec2(0.0, state.banner_and_fallback.size().y + 28.0);
    label(
        &mut draw,
        "CJK and mixed RTL/LTR/numeric text",
        unicode_label,
    );
    draw.rich_text(&state.unicode)
        .position(unicode_label + vec2(0.0, 21.0));

    let center = window_size() - vec2(150.0, 95.0);
    label(&mut draw, "Transform / origin", center - vec2(0.0, 34.0));
    draw.rich_text(&state.transformed)
        .translate(center)
        .origin(Vec2::splat(0.5))
        .rotation(0.35)
        .scale(1.2);
    let bounds = draw.last_text_bounds();
    draw.rect(bounds.min(), bounds.size)
        .stroke_color(Color::AQUA)
        .stroke(1.0);

    // Keeps both setup-owned resources visibly live in this public API example.
    draw.text(&format!("{} retained icons", state.icons.len()))
        .font(&state.font)
        .translate(vec2(28.0, window_size().y - 28.0))
        .size(12.0)
        .color(Color::GRAY);

    gfx::render_to_frame(&draw).unwrap();
}

fn label(draw: &mut draw::Draw2D, text: &str, position: Vec2) {
    draw.text(text)
        .translate(position)
        .size(14.0)
        .color(Color::YELLOW);
}
