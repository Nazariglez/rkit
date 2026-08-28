use rkit::{
    app::{WindowConfig, window_dpi_scale, window_size},
    draw::{self, Draw2D, Font, RenderSprite, RichTextLayout, create_draw_2d, create_draw_2d_for},
    gfx::{self, Color, TextureFilter},
    input::{KeyCode, is_key_pressed},
    math::{Rect, Vec2, vec2},
};

const VIEW_SIZE: Vec2 = vec2(600.0, 400.0);
const AUTO_X: f32 = 35.0;
const FIXED_X: f32 = 325.0;

struct State {
    smooth: Font,
    rich: RichTextLayout,
    pixel: Font,
    target: RenderSprite,
    show_texture: bool,
}

struct Sample<'a> {
    text: &'static str,
    font: &'a Font,
    y: f32,
    size: f32,
    scale: Vec2,
    rotation: f32,
    outline: bool,
}

fn main() -> Result<(), String> {
    rkit::init_with(init)
        .with_window(WindowConfig::default().size(1200, 800))
        .update(update)
        .run()
}

fn init() -> State {
    let smooth = draw::create_font(include_bytes!("assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();
    let pixel = draw::create_font(include_bytes!("assets/press_start_2p.ttf"))
        .with_nearest_filter(true)
        .build()
        .unwrap();
    let rich = draw::rich_text("Retained rich 0123")
        .font(&smooth)
        .size(15.0)
        .layout()
        .unwrap();
    let target = draw::create_render_sprite()
        .with_filter(TextureFilter::Linear)
        .with_size(900, 600)
        .build()
        .unwrap();
    State {
        smooth,
        rich,
        pixel,
        target,
        show_texture: false,
    }
}

fn update(state: &mut State) {
    if is_key_pressed(KeyCode::Space) {
        state.show_texture = !state.show_texture;
    }

    if state.show_texture {
        let mut texture_draw = create_draw_2d_for(&state.target.render_texture);
        texture_draw.set_size(VIEW_SIZE);
        draw_samples(
            &mut texture_draw,
            &state.smooth,
            &state.pixel,
            &state.rich,
            "RENDER TEXTURE: 900x600 target / 600x400 view",
        );
        gfx::render_to_texture(&state.target.render_texture, &texture_draw).unwrap();

        let mut draw = create_draw_2d();
        draw.clear(Color::BLACK);
        let display_size = state.target.render_texture.size() / window_dpi_scale();
        draw.image(&state.target.sprite)
            .position((window_size() - display_size) * 0.5)
            .size(display_size);
        gfx::render_to_frame(&draw).unwrap();
    } else {
        let mut draw = create_draw_2d();
        draw.set_size(VIEW_SIZE);
        draw_samples(
            &mut draw,
            &state.smooth,
            &state.pixel,
            &state.rich,
            "FRAME: 600x400 virtual view",
        );
        gfx::render_to_frame(&draw).unwrap();
    }
}

fn draw_samples(
    draw: &mut Draw2D,
    smooth: &Font,
    pixel: &Font,
    rich: &RichTextLayout,
    title: &str,
) {
    draw.clear(Color::rgb(0.025, 0.03, 0.04));
    draw.text(title)
        .font(smooth)
        .position(vec2(20.0, 12.0))
        .size(9.0)
        .color(Color::GRAY);
    draw.text("AUTOMATIC")
        .font(smooth)
        .position(vec2(AUTO_X, 35.0))
        .size(12.0)
        .color(Color::AQUA);
    draw.text("FIXED resolution(1.0)")
        .font(smooth)
        .position(vec2(FIXED_X, 35.0))
        .size(12.0)
        .resolution(1.0)
        .color(Color::ORANGE);

    for sample in [
        Sample {
            text: "Smooth 0123",
            font: smooth,
            y: 75.0,
            size: 18.0,
            scale: Vec2::ONE,
            rotation: 0.0,
            outline: false,
        },
        Sample {
            text: "Outline 0123",
            font: smooth,
            y: 145.0,
            size: 16.0,
            scale: vec2(1.35, 1.1),
            rotation: -0.04,
            outline: true,
        },
        Sample {
            text: "PIXEL 0123",
            font: pixel,
            y: 225.0,
            size: 10.0,
            scale: Vec2::splat(1.35),
            rotation: 0.0,
            outline: false,
        },
    ] {
        draw_pair(draw, &sample);
    }

    draw_rich_pair(draw, rich, 300.0);
    draw.text("SPACE switches frame / native-size render texture")
        .font(smooth)
        .position(vec2(20.0, 375.0))
        .size(8.0)
        .color(Color::GRAY);
}

fn draw_pair(draw: &mut Draw2D, sample: &Sample<'_>) {
    let automatic = draw_sample(draw, sample, AUTO_X, None, Color::AQUA);
    let fixed = draw_sample(draw, sample, FIXED_X, Some(1.0), Color::ORANGE);
    compare_bounds(automatic, fixed);
    draw_bounds(draw, automatic, Color::AQUA);
    draw_bounds(draw, fixed, Color::ORANGE);
}

fn draw_sample(
    draw: &mut Draw2D,
    sample: &Sample<'_>,
    x: f32,
    resolution: Option<f32>,
    color: Color,
) -> Rect {
    let mut text = draw.text(sample.text);
    text.font(sample.font)
        .translate(vec2(x, sample.y))
        .size(sample.size)
        .scale(sample.scale)
        .rotation(sample.rotation)
        .color(color);
    if sample.outline {
        text.outline(Color::BLACK, 2).shadow_offset(vec2(1.5, 1.5));
    }
    if let Some(resolution) = resolution {
        text.resolution(resolution);
    }
    drop(text);
    draw.last_text_bounds()
}

fn draw_rich_pair(draw: &mut Draw2D, layout: &RichTextLayout, y: f32) {
    let automatic = {
        let mut text = draw.rich_text(layout);
        text.translate(vec2(AUTO_X, y))
            .scale(Vec2::splat(1.2))
            .shadow_offset(vec2(1.5, 1.5));
        drop(text);
        draw.last_text_bounds()
    };
    let fixed = {
        let mut text = draw.rich_text(layout);
        text.translate(vec2(FIXED_X, y))
            .scale(Vec2::splat(1.2))
            .resolution(1.0)
            .shadow_offset(vec2(1.5, 1.5));
        drop(text);
        draw.last_text_bounds()
    };
    compare_bounds(automatic, fixed);
    draw_bounds(draw, automatic, Color::AQUA);
    draw_bounds(draw, fixed, Color::ORANGE);
}

fn compare_bounds(automatic: Rect, fixed: Rect) {
    debug_assert!((automatic.size - fixed.size).abs().max_element() <= 0.01);
}

fn draw_bounds(draw: &mut Draw2D, bounds: Rect, color: Color) {
    draw.rect(bounds.min(), bounds.size)
        .stroke(0.5)
        .stroke_color(color.with_alpha(0.35));
}
