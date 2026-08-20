use rkit::{
    app::{WindowConfig, window_size},
    draw::{self, Font, RichTextBuilder, RichTextLayout, TextIcons, create_draw_2d, text},
    gfx::{self, Color, Texture, TextureFilter},
    math::{Mat3, Rect, Vec2, vec2},
    time,
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
    bidi_controls: RichTextLayout,
    transformed: RichTextLayout,
    dynamic_texture: Texture,
    dynamic_phase: u32,
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
    let pixel = draw::create_sprite()
        .from_image(include_bytes!("./assets/text_icon_pixel.png"))
        .with_filter(TextureFilter::Nearest)
        .build()
        .unwrap();
    let smooth = draw::create_sprite()
        .from_image(include_bytes!("./assets/text_icon_smooth.png"))
        .build()
        .unwrap();
    let banner = draw::create_sprite()
        .from_image(include_bytes!("./assets/text_icon_banner.png"))
        .build()
        .unwrap();
    let banner_left = banner.clone_with_frame(Rect::new(Vec2::ZERO, vec2(16.0, 16.0)));
    let dynamic_pixels = [0x73, 0xEF, 0xF7, 0xFF].repeat(16);
    let dynamic = draw::create_sprite()
        .from_bytes(&dynamic_pixels, 4, 4)
        .with_write_flag(true)
        .with_filter(TextureFilter::Nearest)
        .build()
        .unwrap();
    let dynamic_texture = dynamic.texture().clone();
    let icons = TextIcons::new([
        ("pixel", pixel.clone()),
        ("pixel_copy", pixel),
        ("smooth", smooth),
        ("banner", banner),
        ("banner_left", banner_left),
        ("dynamic", dynamic),
    ])
    .unwrap();

    let default_icon = rich_layout(
        "Nearest [icon:pixel] shared [icon:pixel_copy], linear [icon:smooth], dynamic [icon:dynamic].",
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
        "2:1 [icon:banner size=18], cropped frame [icon:banner_left]. [icon:missing] [icon:pixel size=0] [icon:]",
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
    let bidi_controls = rich_layout(
        "Before: Keepers\u{2068}\u{00A0}\u{2069}[icon:pixel] end\n\
         After: [icon:pixel]\u{2068}\u{00A0}\u{2069} end\n\
         RTL: אבג \u{2066}ABC [icon:pixel]\u{2069} דהו",
        &icons,
        &font,
    )
    .layout()
    .unwrap();
    let bidi_baseline = rich_layout(
        "Before: Keepers\u{00A0}[icon:pixel] end\n\
         After: [icon:pixel]\u{00A0} end\n\
         RTL: אבג ABC [icon:pixel] דהו",
        &icons,
        &font,
    )
    .layout()
    .unwrap();
    assert_eq!(bidi_controls.line_count(), bidi_baseline.line_count());
    for (isolated, baseline) in bidi_controls.lines().iter().zip(bidi_baseline.lines()) {
        assert!(
            (isolated.size().x - baseline.size().x).abs() < 0.01,
            "isolated {:?}, baseline {:?}",
            isolated.size(),
            baseline.size()
        );
    }

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
        bidi_controls,
        transformed,
        dynamic_texture,
        dynamic_phase: 0,
    }
}

fn rich_layout<'a>(content: &'a str, icons: &'a TextIcons, font: &'a Font) -> RichTextBuilder<'a> {
    text::rich_text(content).icons(icons).font(font).size(18.0)
}

fn update(state: &mut State) {
    let phase = time::elapsed_f32() as u32 % 2;
    if state.dynamic_phase != phase {
        state.dynamic_phase = phase;
        let color = if phase == 0 {
            [0x73, 0xEF, 0xF7, 0xFF]
        } else {
            [0xFF, 0x70, 0x70, 0xFF]
        };
        gfx::write_texture(&state.dynamic_texture)
            .from_data(&color.repeat(16))
            .build()
            .unwrap();
    }

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
    let unicode_pos = unicode_label + vec2(0.0, 21.0);
    draw.rich_text(&state.unicode).position(unicode_pos);

    let bidi_label = unicode_pos + vec2(0.0, state.unicode.size().y + 28.0);
    label(
        &mut draw,
        "Bidi controls before/after icons and RTL isolation",
        bidi_label,
    );
    draw.rich_text(&state.bidi_controls)
        .position(bidi_label + vec2(0.0, 21.0));

    let angle = 0.35_f32;
    let scale = 1.2;
    let center = vec2(850.0, 650.0);
    let parent = Mat3::from_translation(vec2(-50.0, -30.0)) * Mat3::from_scale(Vec2::splat(1.1));
    draw.push_matrix(parent);

    label(&mut draw, "Transform / origin", center - vec2(0.0, 34.0));
    draw.rich_text(&state.transformed)
        .translate(center)
        .origin(Vec2::splat(0.5))
        .rotation(angle)
        .scale(scale);
    let bounds = draw.last_text_bounds();
    let scaled_size = state.transformed.size() * scale;
    let (sin, cos) = angle.sin_cos();
    let expected_size = vec2(
        cos.abs() * scaled_size.x + sin.abs() * scaled_size.y,
        sin.abs() * scaled_size.x + cos.abs() * scaled_size.y,
    );
    assert!((bounds.size - expected_size).length() < 0.01);
    draw.rect(bounds.min(), bounds.size)
        .stroke_color(Color::AQUA)
        .stroke(1.0);

    draw.pop_matrix();

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
