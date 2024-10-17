use draw::{AsBindGroups, DrawPipelineId, PipelineContext, PipelineResources};
use rkit::draw::{create_draw_2d, Sprite};
use rkit::gfx::{self, BindGroupLayout, BindingType, BlendMode, Color, VertexFormat, VertexLayout};
use rkit::math::{vec2, Vec2};

// language=wgsl
const SHADER: &str = r#"
struct Transform {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> transform: Transform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uvs: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uvs: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.uvs = model.uvs;
    out.position = transform.mvp * vec4(model.position, 0.0, 1.0);
    return out;
}

@group(1) @binding(0)
var t_texture: texture_2d<f32>;
@group(1) @binding(1)
var s_texture: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var base_color = textureSample(t_texture, s_texture, in.uvs) * in.color;
    let color_tint = vec4<f32>(
        0.5 + 0.5 * sin(in.position.x * 0.1),
        0.5 + 0.5 * cos(in.position.y * 0.1),
        1.0,
        1.0
    );
    return base_color * color_tint;}
"#;

pub fn pixelated_pipeline(res: PipelineResources) -> Result<PipelineContext, String> {
    let pip = gfx::create_render_pipeline(SHADER)
        .with_label("Custom Pipeline")
        // we must use the same vertex layout structure (at least position, uvs, and color, from there we can add)
        .with_vertex_layout(
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x2)
                .with_attr(2, VertexFormat::Float32x4),
        )
        // we must use the same uniform layout as group 0
        .with_bind_group_layout(
            BindGroupLayout::new().with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
        )
        // we must use the same texture/sampler layout as group 1
        .with_bind_group_layout(
            BindGroupLayout::new()
                .with_entry(BindingType::texture(0).with_fragment_visibility(true))
                .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
        )
        .with_blend_mode(BlendMode::NORMAL)
        .build()?;

    let transform_bind_group = gfx::create_bind_group()
        .with_label("Transform")
        .with_layout(pip.bind_group_layout_ref(0)?)
        .with_uniform(0, res.ubo)
        .build()?;

    Ok(PipelineContext {
        pipeline: pip,
        groups: (&[
            transform_bind_group,
            res.sprite_bind_group.clone(), // reuse the binding group for texture/samplers
        ])
            .to_bind_groups(),
        vertex_offset: 8,
        x_pos: 0,
        y_pos: 1,
        alpha_pos: Some(7),
    })
}

struct State {
    ferris: Sprite,
    logo: Sprite,
}

impl State {
    fn new() -> Result<Self, String> {
        let ferris = draw::create_sprite()
            .from_image(include_bytes!("assets/ferris.png"))
            .build()?;

        let logo = draw::create_sprite()
            .from_image(include_bytes!("assets/rust-logo-512x512.png"))
            .build()?;

        // replace the default Images Pipeline with our custom one
        // Any image will use this pipeline from now on
        let _old_shader = draw::set_pipeline_2d(&DrawPipelineId::Images, |res| {
            pixelated_pipeline(res).unwrap()
        });

        Ok(Self { ferris, logo })
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
}

fn update(s: &mut State) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // any image rendered will use our custom shader now

    draw.image(&s.logo)
        .translate(vec2(300.0, 300.0))
        .anchor(Vec2::splat(0.5));

    draw.image(&s.ferris)
        .translate(vec2(600.0, 300.0))
        .anchor(Vec2::splat(0.5));

    gfx::render_to_frame(&draw).unwrap();
}
