use rkit::{draw::{Sprite, create_draw_2d}, prelude::*, gfx::{self, Color}, math::{Vec2, vec2}}};

const COLORS: [Color; 6] = [
    Color::rgb(1.0, 0.0, 0.4666),
    Color::rgb(0.0, 0.3019, 1.0),
    Color::rgb(0.0, 1.0, 0.2039),
    Color::rgb(1.0, 1.0, 0.0),
    Color::rgb(1.0, 0.0941, 0.0),
    Color::rgb(1.0, 0.0, 0.8274),
];

#[derive(Resource)]
struct ColorSprites {
    control: Sprite,
    sprite: Sprite,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_setup(setup_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    let control = draw::create_sprite()
        .from_image(include_bytes!("assets/colors.png"))
        .build()
        .unwrap();

    let sprite = init_rect();

    cmds.insert_resource(ColorSprites { control, sprite });
}

fn draw_system(sprites: Res<ColorSprites>, window: Res<Window>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(1.0, 0.0, 0.4666));

    draw.image(&sprites.control).position(Vec2::Y * 100.0);

    COLORS.into_iter().enumerate().for_each(|(i, c)| {
        draw.rect(vec2(100.0 * i as f32, 0.0), Vec2::splat(100.0))
            .color(c);

        draw.image(&sprites.sprite)
            .position(vec2(100.0 * i as f32, 200.0))
            .color(c);

        draw.text(&i.to_string())
            .position(vec2(100.0 * i as f32 + 50.0, 150.0))
            .anchor(Vec2::splat(0.5))
            .size(30.0)
            .color(c);
    });

    draw.text("If you cannot see the letters on the top, the shapes or the images, then this example is working. It's meant to be a control example for the Draw2D Srgb Color to Linar when rendering.")
        .position(window.size() * 0.5 + Vec2::Y * 200.0)
        .anchor(Vec2::splat(0.5))
        .size(14.0)
        .max_width(600.0)
        .h_align_center()
        .color(Color::BLACK);

    gfx::render_to_frame(&draw).unwrap();
}

fn init_rect() -> Sprite {
    let rt = gfx::create_render_texture()
        .with_size(100, 100)
        .build()
        .unwrap();

    let mut draw = create_draw_2d();
    draw.clear(Color::WHITE);
    draw.rect(Vec2::ZERO, Vec2::splat(100.0))
        .color(Color::WHITE);
    gfx::render_to_texture(&rt, &draw).unwrap();

    draw::create_sprite()
        .from_texture(&rt.texture())
        .build()
        .unwrap()
}
