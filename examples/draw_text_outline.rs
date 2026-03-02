use rkit::{
    draw::{self, Font, create_draw_2d},
    gfx::{self, Color},
    math::vec2,
    prelude::*,
};

struct State {
    pixel_font: Font,
}

fn main() -> Result<(), String> {
    rkit::init_with(init).update(update).run()
}

fn init() -> State {
    let pixel_font = draw::create_font(include_bytes!("./assets/press_start_2p.ttf"))
        .with_nearest_filter(true)
        .build()
        .unwrap();
    State { pixel_font }
}

fn update(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.set_round_pixels(true);
    draw.clear(Color::rgb(0.1, 0.2, 0.3));

    draw.text("KARMA KEEPERS")
        .font(&s.pixel_font)
        .position(vec2(20.0, 20.0))
        .size(64.0)
        .outline(Color::RED, 2)
        .shadow_color(Color::BLACK)
        .shadow_offset(vec2(2.0, 2.0));

    // draw.text("Default font, 2px outline")
    //     .position(vec2(20.0, 20.0))
    //     .size(24.0)
    //     .color(Color::WHITE)
    //     .outline(Color::BLACK, 2);

    // draw.text("PIXEL FONT 1PX")
    //     .font(&s.pixel_font)
    //     .position(vec2(20.0, 70.0))
    //     .size(16.0)
    //     .color(Color::YELLOW)
    //     .outline(Color::BLACK, 1);

    // draw.text("PIXEL FONT 2PX")
    //     .font(&s.pixel_font)
    //     .position(vec2(20.0, 110.0))
    //     .size(16.0)
    //     .color(Color::ORANGE)
    //     .outline(Color::BLACK, 2);

    // draw.text("BLUE OUTLINE")
    //     .font(&s.pixel_font)
    //     .position(vec2(20.0, 160.0))
    //     .size(16.0)
    //     .color(Color::WHITE)
    //     .outline(Color::NAVY, 1);

    // draw.text("OUTLINE + SHADOW")
    //     .font(&s.pixel_font)
    //     .position(vec2(20.0, 210.0))
    //     .size(16.0)
    //     .color(Color::WHITE)
    //     .outline(Color::BLACK, 1)
    //     .shadow_offset(vec2(2.0, 2.0))
    //     .shadow_color(Color::rgba(0.0, 0.0, 0.5, 0.8));

    // draw.text("This longer text wraps and every glyph still has its outline.")
    //     .font(&s.pixel_font)
    //     .position(vec2(20.0, 270.0))
    //     .size(16.0)
    //     .color(Color::WHITE)
    //     .outline(Color::BLACK, 1)
    //     .max_width(280.0);

    // draw.text("CENTER ALIGNED")
    //     .font(&s.pixel_font)
    //     .position(vec2(500.0, 70.0))
    //     .size(16.0)
    //     .color(Color::MAGENTA)
    //     .outline(Color::BLACK, 1)
    //     .h_align_center()
    //     .anchor(vec2(0.5, 0.0));

    // draw.text("RIGHT ALIGNED")
    //     .font(&s.pixel_font)
    //     .position(vec2(740.0, 110.0))
    //     .size(16.0)
    //     .color(Color::rgb(0.0, 1.0, 1.0))
    //     .outline(Color::BLACK, 1)
    //     .h_align_right()
    //     .anchor(vec2(1.0, 0.0));

    // draw.text("BOUNDS CHECK")
    //     .font(&s.pixel_font)
    //     .position(vec2(20.0, 420.0))
    //     .size(24.0)
    //     .color(Color::WHITE)
    //     .outline(Color::RED, 1);

    // let bounds = draw.last_text_bounds();
    // draw.rect(bounds.min(), bounds.size)
    //     .stroke_color(Color::GREEN)
    //     .stroke(1.0);

    // draw.text("NO OUTLINE (compare)")
    //     .font(&s.pixel_font)
    //     .position(vec2(20.0, 470.0))
    //     .size(24.0)
    //     .color(Color::WHITE);

    // let bounds = draw.last_text_bounds();
    // draw.rect(bounds.min(), bounds.size)
    //     .stroke_color(Color::GREEN)
    //     .stroke(1.0);

    gfx::render_to_frame(&draw).unwrap();
}
