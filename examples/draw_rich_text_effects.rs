use std::sync::Arc;

use rkit::{
    app::WindowConfig,
    draw::{self, RichTextDocument, RichTextLayout, TextEffect, TextEffects, create_draw_2d, text},
    gfx::{self, Color},
    math::vec2,
    time,
};

struct State {
    markup: RichTextLayout,
    document: RichTextLayout,
}

fn main() -> Result<(), String> {
    rkit::init_with(init)
        .with_window(WindowConfig::default().size(900, 320))
        .update(update)
        .run()
}

fn init() -> State {
    let font = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();
    let nested = Arc::new(
        text::rich_text("Nested draw")
            .font(&font)
            .size(12.0)
            .layout()
            .unwrap(),
    );
    let nested_wave = nested.clone();
    let effects = TextEffects::new([
        (
            "wave",
            TextEffect::new(move |mut run| {
                let _ = draw::text_metrics("callback measurement").measure();
                let mut nested_draw = create_draw_2d();
                nested_draw.rich_text(&nested_wave);
                let time = run.time();
                for mut item in run.items_mut() {
                    let y = (time * 4.0 + item.index() as f32 * 0.5).sin() * 8.0;
                    item.translate(vec2(0.0, y)).rotate(y * 0.01);
                }
            }),
        ),
        (
            "pulse",
            TextEffect::new(|mut run| {
                let scale = 1.0 + run.time().sin() * 0.15;
                for mut item in run.items_mut() {
                    item.scale(scale).tint(Color::YELLOW);
                }
            }),
        ),
        (
            "colors",
            TextEffect::new(|mut run| {
                let palette = [Color::MAGENTA, Color::AQUA, Color::YELLOW, Color::WHITE];
                let phase = (run.time() * 4.0) as usize;
                for mut item in run.items_mut() {
                    item.set_color(palette[(item.index() + phase) % palette.len()]);
                }
            }),
        ),
    ])
    .unwrap();
    let markup = text::rich_text(
        "[effect:wave][u]Wave across spaces[/u][/effect]\n[effect:pulse]Nested [effect:wave]effects[/effect][/effect]\n[effect:colors]Color swap per letter[/effect]",
    )
    .font(&font)
    .effects(&effects)
    .size(34.0)
    .layout()
    .unwrap();
    let mut content = RichTextDocument::new();
    content.effect("wave", |content| {
        content.text("Programmatic effect scope");
    });
    let document = text::rich_document(&content)
        .font(&font)
        .effects(&effects)
        .size(28.0)
        .layout()
        .unwrap();
    State { markup, document }
}

fn update(state: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    let now = time::elapsed_f32();
    draw.rich_text(&state.markup)
        .position(vec2(50.0, 50.0))
        .effect_time(now)
        .effect_seed(7);
    draw.rich_text(&state.document)
        .position(vec2(50.0, 210.0))
        .effect_time(now)
        .effect_seed(7);
    gfx::render_to_frame(&draw).unwrap();
}
