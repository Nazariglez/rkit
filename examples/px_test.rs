use std::io::Read;

use draw::{
    Draw2D, RenderSprite, Sprite, Transform2D, create_draw_2d, create_render_sprite, create_sprite,
    text_mask_atlas,
};
use rkit::{
    gfx::{self, Color, TextureFilter, TextureFormat},
    math::{Mat3, Vec2, vec2},
    prelude::*,
};

const RES: f32 = 1.0;

#[derive(Resource)]
struct State {
    tex: Sprite,
    rt: RenderSprite,
    rtex: Sprite,
    rrt: RenderSprite,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(WindowConfigPlugin::default().pixelated(true).size(800, 900))
        .on_setup(setup)
        .on_render(render)
        .run()
}

fn setup(mut cmds: Commands) {
    let sprite = create_sprite()
        .from_image(include_bytes!("./assets/px_test.png"))
        .with_filter(TextureFilter::Nearest)
        // .with_format(TextureFormat::R8UNorm)
        .build()
        .unwrap();

    let size = sprite.size() * RES;

    let rt = create_render_sprite()
        .with_filter(TextureFilter::Nearest)
        .with_size(size.x as _, size.y as _)
        .build()
        .unwrap();

    let rgba = image::load_from_memory(include_bytes!("./assets/px_test.png"))
        .unwrap()
        .to_rgba8()
        .bytes()
        .step_by(4)
        .map(|b| b.unwrap())
        .collect::<Vec<u8>>();

    let rtex = create_sprite()
        .from_bytes(&rgba, sprite.width() as _, sprite.height() as _)
        .with_filter(TextureFilter::Nearest)
        .with_format(TextureFormat::R8UNorm)
        .build()
        .unwrap();

    let rrt = create_render_sprite()
        .with_filter(TextureFilter::Nearest)
        .with_size(size.x as _, size.y as _)
        .build()
        .unwrap();

    cmds.insert_resource(State {
        tex: sprite,
        rt,
        rtex,
        rrt,
    });
}

fn render(state: Res<State>) {
    let px = 8.0;
    let word = "test hola";
    let mut rt_draw = Draw2D::new(state.rt.render_texture.size());
    rt_draw.clear(Color::TRANSPARENT);
    rt_draw.push_matrix(Mat3::from_scale(Vec2::splat(RES)));
    rt_draw.image(&state.tex);
    rt_draw
        .text(&format!("1.{word}"))
        .translate(vec2(36.0, 2.0))
        .size(px);
    gfx::render_to_texture(&state.rt.render_texture, &rt_draw).unwrap();

    let mut rrt_draw = Draw2D::new(state.rrt.render_texture.size());
    rrt_draw.clear(Color::TRANSPARENT);
    rrt_draw.push_matrix(Mat3::from_scale(Vec2::splat(RES)));
    rrt_draw.image(&state.rtex);
    rrt_draw
        .text(&format!("2.{word}"))
        .translate(vec2(36.0, 2.0))
        .size(px);
    gfx::render_to_texture(&state.rrt.render_texture, &rrt_draw).unwrap();

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    draw.push_matrix(
        Transform2D::builder()
            .set_scale(Vec2::splat(4.0))
            .build()
            .as_mat3(),
    );
    draw.image(&state.tex).translate(vec2(2.0, 10.0));
    draw.text(&format!("3.{word}"))
        .translate(vec2(38.0, 12.0))
        .size(px);
    draw.image(&state.rt.sprite)
        .translate(vec2(100.0, 10.0))
        .scale(Vec2::splat(1.0 / RES));
    draw.image(&state.rtex).translate(vec2(2.0, 48.0));
    draw.text(&format!("4.{word}"))
        .translate(vec2(38.0, 50.0))
        .size(px);
    draw.image(&state.rrt.sprite)
        .translate(vec2(100.0, 48.0))
        .scale(Vec2::splat(1.0 / RES));
    draw.pop_matrix();

    let atlas = create_sprite()
        .from_texture(&text_mask_atlas())
        .with_filter(TextureFilter::Nearest)
        .with_format(TextureFormat::R8UNorm)
        .build()
        .unwrap();

    let atlas_pos = vec2(10.0, 340.0);
    draw.push_matrix(
        Transform2D::builder()
            .set_translation(atlas_pos)
            .set_scale(Vec2::splat(2.0))
            .build()
            .as_mat3(),
    );
    draw.image(&atlas);
    draw.rect(-Vec2::ONE, atlas.size() - Vec2::ONE)
        .alpha(0.1)
        .stroke(1.0);
    draw.pop_matrix();

    gfx::render_to_frame(&draw).unwrap();
}
