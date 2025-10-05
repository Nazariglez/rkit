use rkit::{
    app::window_size,
    draw::{self, Draw2D, RenderSprite, create_draw_2d},
    gfx::{self, AsRenderer, Color},
    math::Vec2,
};

struct State {
    render_sprite: RenderSprite,
}

fn main() -> Result<(), String> {
    rkit::init_with(init).update(update).run()
}

fn init() -> State {
    // Create a render sprite for testing text on render textures
    let render_sprite = draw::create_render_sprite()
        .with_filter(gfx::TextureFilter::Nearest)
        .with_size(400, 300)
        .build()
        .unwrap();

    State { render_sprite }
}

fn update(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::new(0.1, 0.1, 0.1, 1.0));

    let screen_size = window_size();

    // First, draw text to the render sprite
    {
        let mut render_draw = Draw2D::new(Vec2::new(400.0, 300.0));
        render_draw.clear(Color::new(0.05, 0.05, 0.05, 1.0)); // Dark background for contrast

        // Example 1: Regular text on render texture (old behavior)
        render_draw
            .text("Regular on RT")
            .size(8.0)
            .translate(Vec2::new(20.0, 20.0))
            .color(Color::WHITE);

        // Example 2: Pixel perfect text on render texture (new behavior)
        render_draw
            .text("Pixel Perfect on RT")
            .size(8.0)
            .translate(Vec2::new(20.0, 50.0))
            .color(Color::GREEN);

        // Example 3: Small text on render texture
        render_draw
            .text("Small on RT")
            .size(4.0)
            .translate(Vec2::new(20.0, 80.0))
            .color(Color::new(0.0, 1.0, 1.0, 1.0));

        // Example 4: Very small text on render texture
        render_draw
            .text("Tiny on RT")
            .size(3.0)
            .translate(Vec2::new(20.0, 110.0))
            .color(Color::new(1.0, 0.0, 1.0, 1.0));

        // Example 5: Scaled text on render texture
        render_draw
            .text("Scaled on RT")
            .size(16.0)
            .translate(Vec2::new(200.0, 150.0))
            .color(Color::YELLOW)
            .scale(Vec2::new(0.5, 0.5));

        // Example 6: Multiple sizes to show the difference
        for (i, size) in [4.0, 6.0, 8.0, 12.0].iter().enumerate() {
            let y = 150.0 + (i as f32 * 25.0);
            render_draw
                .text(&format!("Size {}px", size))
                .size(*size)
                .translate(Vec2::new(20.0, y))
                .color(Color::WHITE);
        }

        // Render the draw commands to the render texture
        render_draw
            .render(Some(&s.render_sprite.render_texture))
            .unwrap();
    }

    // Now draw the render sprite to the screen with different scales
    draw.image(&s.render_sprite.sprite)
        .translate(Vec2::new(50.0, 50.0))
        .scale(Vec2::new(1.0, 1.0)); // Original size

    draw.image(&s.render_sprite.sprite)
        .translate(Vec2::new(500.0, 50.0))
        .scale(Vec2::new(0.5, 0.5)); // Half size

    draw.image(&s.render_sprite.sprite)
        .translate(Vec2::new(700.0, 50.0))
        .scale(Vec2::new(0.25, 0.25)); // Quarter size

    // Add labels to show what we're looking at
    draw.text("Original Size")
        .size(8.0)
        .translate(Vec2::new(50.0, 30.0))
        .color(Color::WHITE);

    draw.text("Half Size")
        .size(8.0)
        .translate(Vec2::new(500.0, 30.0))
        .color(Color::WHITE);

    draw.text("Quarter Size")
        .size(8.0)
        .translate(Vec2::new(700.0, 30.0))
        .color(Color::WHITE);

    // Instructions
    draw.text("Render Texture Test:")
        .size(10.0)
        .translate(Vec2::new(50.0, screen_size.y - 150.0))
        .color(Color::YELLOW);

    draw.text("Text is drawn to a render texture, then scaled down")
        .size(8.0)
        .translate(Vec2::new(50.0, screen_size.y - 130.0))
        .color(Color::WHITE);

    draw.text("Green text uses pixel_perfect(true) - stays crisp!")
        .size(8.0)
        .translate(Vec2::new(50.0, screen_size.y - 110.0))
        .color(Color::GREEN);

    draw.text("White text uses default rendering - gets blurry when scaled")
        .size(8.0)
        .translate(Vec2::new(50.0, screen_size.y - 90.0))
        .color(Color::WHITE);

    draw.text("This demonstrates the improvement for render textures!")
        .size(8.0)
        .translate(Vec2::new(50.0, screen_size.y - 70.0))
        .color(Color::new(1.0, 0.5, 0.0, 1.0));

    gfx::render_to_frame(&draw).unwrap();
}
