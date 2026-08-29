use rkit::{
    app::WindowConfig,
    draw::{self, Font, RichTextLayout, TextIcons, create_draw_2d, text},
    gfx::{AsRenderer, Color, TextureFilter},
    math::vec2,
};

struct State {
    font: Font,
    layouts: Vec<(&'static str, RichTextLayout)>,
}

fn main() -> Result<(), String> {
    rkit::init_with(init)
        .with_window(WindowConfig::default().size(820, 420))
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
    let banner = draw::create_sprite()
        .from_image(include_bytes!("./assets/text_icon_banner.png"))
        .with_filter(TextureFilter::Nearest)
        .build()
        .unwrap();
    let icons = TextIcons::new([("pixel", pixel), ("banner", banner)]).unwrap();
    let styles = text::TextStyles::default();
    let rows = [
        (
            "default",
            "Agjp [icon:pixel size=36] [icon:banner size=36] Agjp",
        ),
        (
            "middle",
            "Agjp [icon:pixel size=36 align=middle] [icon:banner size=36 align=middle] Agjp",
        ),
        (
            "baseline",
            "Agjp [icon:pixel size=36 align=baseline] [icon:banner size=36 align=baseline] Agjp",
        ),
        (
            "top",
            "Agjp [icon:pixel size=36 align=top] [icon:banner size=36 align=top] Agjp",
        ),
        (
            "bottom",
            "Agjp [icon:pixel size=36 align=bottom] [icon:banner size=36 align=bottom] Agjp",
        ),
    ];
    let layouts = rows
        .into_iter()
        .map(|(label, content)| {
            let layout = text::rich_text(content)
                .icons(&icons)
                .styles(&styles)
                .font(&font)
                .size(22.0)
                .line_height(44.0)
                .layout()
                .unwrap();
            (label, layout)
        })
        .collect();
    State { font, layouts }
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.06, 0.07, 0.1));
    for (index, (label, layout)) in state.layouts.iter().enumerate() {
        let y = 30.0 + index as f32 * 70.0;
        draw.text(label)
            .font(&state.font)
            .size(16.0)
            .position(vec2(24.0, y + 12.0));
        draw.rect(vec2(130.0, y), layout.size())
            .stroke_color(Color::rgb(0.35, 0.4, 0.5))
            .stroke(1.0);
        draw.rich_text(layout).position(vec2(130.0, y));
    }
    draw.render(None).unwrap();
}
