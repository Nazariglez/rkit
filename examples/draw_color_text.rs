use rkit::{
    draw::create_draw_2d,
    gfx::{self, Color},
    math::{Vec2, vec2},
    prelude::*,
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_render(draw_system)
        .run()
}

fn draw_system(time: Res<Time>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.1, 0.1, 0.15));

    draw.text("Hello [color:#FF0000]RED[/color] and [color:#00FF00]GREEN[/color] world!")
        .color_tags()
        .position(vec2(50.0, 50.0))
        .size(24.0)
        .color(Color::WHITE);

    // nested colors
    draw.text("Normal [color:#FF8800]orange [color:#00FFFF]cyan inside[/color] back to orange[/color] normal")
        .color_tags()
        .position(vec2(50.0, 100.0))
        .size(18.0)
        .color(Color::SILVER);

    // wrapping with colors
    let wrapped_text = "This is a [color:#FF0000]long text[/color] that will [color:#00FF00]wrap automatically[/color] when it reaches the [color:#0088FF]maximum width[/color] specified. The colors should [color:#FFFF00]flow correctly[/color] across lines.";
    draw.text(wrapped_text)
        .color_tags()
        .position(vec2(50.0, 160.0))
        .size(16.0)
        .color(Color::WHITE)
        .max_width(300.0);

    // center align with colors
    draw.text("[color:#FF00FF]Center[/color] aligned [color:#00FFFF]rich text[/color] with wrapping")
        .color_tags()
        .position(vec2(550.0, 160.0))
        .size(16.0)
        .color(Color::WHITE)
        .max_width(200.0)
        .h_align_center()
        .anchor(vec2(0.5, 0.0));

    // right align
    draw.text("[color:#FFFF00]Right[/color] aligned text")
        .color_tags()
        .position(vec2(750.0, 160.0))
        .size(16.0)
        .color(Color::WHITE)
        .max_width(200.0)
        .h_align_right()
        .anchor(vec2(1.0, 0.0));

    // with shadow
    draw.text("Text with [color:#FF0000]colored[/color] shadow")
        .color_tags()
        .position(vec2(50.0, 300.0))
        .size(32.0)
        .color(Color::WHITE)
        .shadow_offset(vec2(3.0, 3.0))
        .shadow_color(Color::rgb(0.0, 0.0, 0.3));

    // with transform
    let angle = time.elapsed_f32();
    draw.text("[color:#FF0000]R[/color][color:#FF7F00]O[/color][color:#FFFF00]T[/color][color:#00FF00]A[/color][color:#0000FF]T[/color][color:#8B00FF]E[/color]")
        .color_tags()
        .position(vec2(600.0, 350.0))
        .size(48.0)
        .color(Color::WHITE)
        .origin(0.5)
        .rotation(angle);

    let scale = 1.0 + 0.3 * (time.elapsed_f32() * 2.0).sin();
    draw.text("[color:#00FFFF]Pulsing[/color] [color:#FF00FF]Scale[/color]")
        .color_tags()
        .position(vec2(600.0, 450.0))
        .size(24.0)
        .color(Color::WHITE)
        .anchor(Vec2::splat(0.5))
        .scale(Vec2::splat(scale));

    // with alpha
    draw.text("Normal [color:#FF000080]50% red[/color] and [color:#00FF0040]25% green[/color] transparency")
        .color_tags()
        .position(vec2(50.0, 400.0))
        .size(20.0)
        .color(Color::WHITE);

    // another with max width
    let paragraph = "[color:#FFD700]Lorem ipsum[/color] dolor sit amet, [color:#FF6347]consectetur adipiscing[/color] elit. Sed do [color:#7B68EE]eiusmod tempor[/color] incididunt ut labore et [color:#3CB371]dolore magna aliqua[/color]. Ut enim ad minim veniam, [color:#FF69B4]quis nostrud exercitation[/color] ullamco laboris.";
    draw.text(paragraph)
        .color_tags()
        .position(vec2(50.0, 480.0))
        .size(14.0)
        .color(Color::WHITE)
        .max_width(700.0);

    gfx::render_to_frame(&draw).unwrap();
}
