use rkit::app::window_size;
use rkit::draw::{self, Font, create_draw_2d};
use rkit::gfx::{self, Color};
use rkit::math::{Vec2, vec2};

struct State {
    font1: Font,
    font2: Font,
}

fn main() -> Result<(), String> {
    rkit::init_with(init).update(update).run()
}

fn init() -> State {
    let font1 = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();

    let font2 = draw::create_font(include_bytes!("./assets/kenney_pixel-webfont.ttf"))
        .with_nearest_filter(true)
        .build()
        .unwrap();
    State { font1, font2 }
}

fn update(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let yy_multi = 0.05;
    let count = 15;
    let gap = 25.0;
    let pos1 = window_size() * vec2(0.2, yy_multi);
    let offset = vec2(0.0, 30.0);
    for i in 0..count {
        let size = 8.0 + (i * 2) as f32;
        let pos = (pos1 + offset * i as f32 + (vec2(0.0, gap) * i as f32)).floor();
        draw.text(&format!("{size} font."))
            .h_align_center()
            .translate(pos)
            .color(Color::YELLOW)
            .size(size)
            // .resolution(8.0)
            .origin(Vec2::splat(0.5));

        let bounds = draw.last_text_bounds();
        draw.rect(bounds.min(), bounds.size)
            .stroke_color(Color::RED)
            .stroke(1.0);
    }

    let pos2 = window_size() * vec2(0.5, yy_multi);
    for i in 0..count {
        let size = 8.0 + (i * 2) as f32;
        let pos = (pos2 + offset * i as f32 + (vec2(0.0, gap) * i as f32)).floor();
        draw.text(&format!("{size} font."))
            .font(&s.font2)
            .h_align_center()
            .translate(pos)
            .color(Color::YELLOW)
            .size(size)
            // .resolution(8.0)
            .origin(Vec2::splat(0.5));

        let bounds = draw.last_text_bounds();
        draw.rect(bounds.min(), bounds.size)
            .stroke_color(Color::RED)
            .stroke(1.0);
    }

    let pos3 = window_size() * vec2(0.80, yy_multi);
    for i in 0..count {
        let size = 8.0 + (i * 2) as f32;
        let pos = (pos3 + offset * i as f32 + (vec2(0.0, gap) * i as f32)).floor();
        draw.text(&format!("{size} font."))
            .font(&s.font1)
            .h_align_center()
            .translate(pos)
            .color(Color::YELLOW)
            .size(size)
            // .resolution(8.0)
            .origin(Vec2::splat(0.5));

        let bounds = draw.last_text_bounds();
        draw.rect(bounds.min(), bounds.size)
            .stroke_color(Color::RED)
            .stroke(1.0);
    }

    for i in 0..count {
        let pos =
            (window_size() * vec2(0.5, yy_multi) + offset * i as f32 + (vec2(0.0, gap) * i as f32))
                .floor();
        draw.line(vec2(20.0, pos.y), vec2(window_size().x - 20.0, pos.y))
            .color(Color::BLUE)
            .width(1.0);
    }

    draw.line(vec2(pos1.x, 10.0), vec2(pos1.x, window_size().y - 20.0))
        .color(Color::BLUE)
        .width(1.0);

    draw.line(vec2(pos2.x, 10.0), vec2(pos2.x, window_size().y - 20.0))
        .color(Color::BLUE)
        .width(1.0);

    draw.line(vec2(pos3.x, 10.0), vec2(pos3.x, window_size().y - 20.0))
        .color(Color::BLUE)
        .width(1.0);

    gfx::render_to_frame(&draw).unwrap();
}
