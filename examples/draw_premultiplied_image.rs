use rkit::{
    draw::{
        Draw2D, RenderSprite, RenderSpriteCache, Sprite, batch::SpriteBatcher, create_draw_2d,
        create_draw_2d_for, create_render_sprite, create_sprite,
    },
    gfx::{self, Color, Renderer},
    math::{Rect, UVec2, Vec2, orthographic},
    prelude::*,
};

#[derive(Resource)]
struct State {
    layer: RenderSprite,
    raw_pm_layer: Sprite,
    pm_frame: Sprite,
    straight_layer: Sprite,
    cache: RenderSpriteCache,
    png: Sprite,
}

#[derive(Resource, Deref)]
struct Batcher(SpriteBatcher);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .on_setup(setup)
        .on_render(draw_system)
        .run()
}

fn setup(mut commands: Commands) {
    let layer = create_render_sprite()
        .with_label("Premultiplied image example layer")
        .with_size(220, 140)
        .build()
        .or_panic("Creating premultiplied image example layer");
    let raw_pm_layer = create_sprite()
        .from_texture(layer.render_texture.texture())
        .with_sampler(layer.sprite.sampler())
        .with_premultiplied_source()
        .build()
        .or_panic("Creating premultiplied wrapper for premultiplied image example layer");
    let pm_frame = raw_pm_layer.clone_with_frame(Rect::new(Vec2::ZERO, Vec2::new(110.0, 140.0)));
    let straight_layer = create_sprite()
        .from_texture(layer.render_texture.texture())
        .with_sampler(layer.sprite.sampler())
        .build()
        .or_panic("Creating straight wrapper for premultiplied image example layer");
    let cache = RenderSpriteCache::new(1, layer.sprite.sampler().clone())
        .or_panic("Creating premultiplied image example RenderSpriteCache");
    let png = create_sprite()
        .from_image(include_bytes!("assets/ferris.png"))
        .build()
        .or_panic("Creating premultiplied image example PNG");
    commands.insert_resource(State {
        layer,
        raw_pm_layer,
        pm_frame,
        straight_layer,
        cache,
        png,
    });
    commands.insert_resource(Batcher(
        SpriteBatcher::new().or_panic("Creating premultiplied image example SpriteBatcher"),
    ));
}

fn translucent_circles(draw: &mut Draw2D, at: Vec2) {
    draw.circle(52.0)
        .position(at + Vec2::new(78.0, 72.0))
        .fill_color(Color::rgb(0.2, 0.85, 0.85).with_alpha(0.45))
        .fill();
    draw.circle(52.0)
        .position(at + Vec2::new(142.0, 72.0))
        .fill_color(Color::MAGENTA.with_alpha(0.45))
        .fill();
}

fn draw_system(mut state: ResMut<State>, window: Res<Window>, mut batcher: ResMut<Batcher>) {
    let State {
        layer,
        raw_pm_layer,
        pm_frame,
        straight_layer,
        cache,
        png,
    } = &mut *state;

    let mut layer_draw = create_draw_2d_for(&layer.render_texture);
    layer_draw.clear(Color::TRANSPARENT);
    translucent_circles(&mut layer_draw, Vec2::ZERO);
    gfx::render_to_texture(&layer.render_texture, &layer_draw)
        .or_panic("Rendering premultiplied image example layer");

    let layer_sprite = &layer.sprite;
    let cached_layer = cache.get(UVec2::new(220, 140));
    let mut nested_layer = create_draw_2d_for(&cached_layer.render_texture);
    nested_layer.clear(Color::TRANSPARENT);
    nested_layer
        .image(layer_sprite)
        .size(Vec2::new(220.0, 140.0));
    gfx::render_to_texture(&cached_layer.render_texture, &nested_layer)
        .or_panic("Rendering cached nested premultiplied image example layer");

    let origin = (window.size() - Vec2::new(480.0, 500.0)).max(Vec2::ZERO) * 0.5;
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.08, 0.1, 0.13));

    draw.rect(origin, Vec2::new(220.0, 140.0))
        .fill_color(Color::WHITE)
        .fill();
    draw.image(layer_sprite)
        .position(origin)
        .size(Vec2::new(220.0, 140.0));

    // The same translucent circles drawn directly beside the RT composite:
    // both must look identical.
    let direct = origin + Vec2::new(250.0, 0.0);
    draw.rect(direct, Vec2::new(220.0, 140.0))
        .fill_color(Color::WHITE)
        .fill();
    translucent_circles(&mut draw, direct);

    let nested = origin + Vec2::new(0.0, 170.0);
    draw.rect(nested, Vec2::new(220.0, 140.0))
        .fill_color(Color::rgb(0.12, 0.15, 0.2))
        .fill();
    draw.pattern(pm_frame)
        .position(nested)
        .size(Vec2::new(220.0, 140.0))
        .alpha(0.2);
    draw.set_alpha(0.5);
    draw.push_rounded_clip(Rect::new(nested, Vec2::new(220.0, 140.0)), 20.0);
    // This cached layer is composited through two transparent RenderSprites.
    draw.image(&cached_layer.sprite)
        .position(nested)
        .size(Vec2::new(220.0, 140.0))
        .color(Color::YELLOW.with_alpha(0.75))
        .alpha(0.6);
    draw.pop_clip();
    draw.set_alpha(1.0);

    // Zero inherited alpha must hide the composite entirely.
    let hidden = origin + Vec2::new(250.0, 170.0);
    draw.rect(hidden, Vec2::new(220.0, 140.0))
        .fill_color(Color::rgb(0.12, 0.15, 0.2))
        .fill();
    draw.set_alpha(0.0);
    draw.image(layer_sprite)
        .position(hidden)
        .size(Vec2::new(220.0, 140.0));
    draw.set_alpha(1.0);

    let source_origin = origin + Vec2::new(0.0, 340.0);
    let source_size = Vec2::new(99.0, 63.0);
    draw.text("Draw2D: PM / straight / PM frame crop")
        .size(9.0)
        .position(source_origin - Vec2::new(0.0, 12.0));
    for offset in [0.0, 120.0, 240.0] {
        draw.rect(source_origin + Vec2::new(offset, 0.0), source_size)
            .fill_color(Color::WHITE)
            .fill();
    }
    draw.image(layer_sprite)
        .position(source_origin)
        .size(source_size);
    draw.image(straight_layer)
        .position(source_origin + Vec2::new(120.0, 0.0))
        .size(source_size);
    draw.image(pm_frame)
        .position(source_origin + Vec2::new(240.0, 0.0))
        .size(source_size)
        .crop(Vec2::ZERO, Vec2::new(55.0, 140.0));

    draw.text("Ordinary PNG")
        .size(9.0)
        .position(origin + Vec2::new(370.0, 328.0));
    draw.image(png)
        .position(origin + Vec2::new(370.0, 340.0))
        .alpha(0.7);

    let batch_origin = origin + Vec2::new(0.0, 430.0);
    let batch_size = Vec2::new(99.0, 63.0);
    draw.text("SpriteBatcher: PM source with color alpha")
        .size(9.0)
        .position(batch_origin - Vec2::new(0.0, 12.0));
    draw.text("same texture, straight declaration")
        .size(9.0)
        .position(batch_origin + Vec2::new(120.0, -12.0));
    draw.rect(batch_origin, batch_size)
        .fill_color(Color::WHITE)
        .fill();
    draw.rect(batch_origin + Vec2::new(120.0, 0.0), batch_size)
        .fill_color(Color::WHITE)
        .fill();

    gfx::render_to_frame(&draw).or_panic("Rendering premultiplied image example");

    batcher.clear();
    batcher.set_projection(orthographic(
        0.0,
        window.size().x,
        window.size().y,
        0.0,
        0.0,
        1.0,
    ));
    let batch_center = batch_origin + batch_size * 0.5;
    batcher
        .sprite(raw_pm_layer, batch_center)
        .scale(Vec2::new(0.45, 0.45))
        .color(Color::WHITE.with_alpha(0.6));
    batcher
        .sprite(straight_layer, batch_center + Vec2::new(120.0, 0.0))
        .scale(Vec2::new(0.45, 0.45))
        .color(Color::WHITE.with_alpha(0.6));
    batcher
        .upload()
        .or_panic("Uploading premultiplied image example SpriteBatcher");

    let mut renderer = Renderer::new();
    batcher.apply_pass_to(&mut renderer);
    gfx::render_to_frame(&renderer).or_panic("Rendering premultiplied image example SpriteBatcher");
}
