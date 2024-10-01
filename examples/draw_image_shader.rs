use draw::{AsBindGroups, DrawPipelineId, PipelineContext, PipelineResources};
use rkit::draw::{create_draw_2d, Sprite};
use rkit::gfx::{
    self, BindGroupLayout, BindingType, BlendMode, Buffer, Color, VertexFormat, VertexLayout,
};
use rkit::math::{vec2, Vec2};
use rkit::time;

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

struct PixelData {
    size: f32,
};

@group(2) @binding(0)
var<uniform> pixel_data: PixelData;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
let tex_size = vec2<f32>(textureDimensions(t_texture));
    let pixel_size = vec2<f32>(pixel_data.size);
    let shifted_uvs = in.uvs - 0.5;
    let coords = floor((shifted_uvs * tex_size) / pixel_size) * pixel_size;
    let uvs = (coords / tex_size) + 0.5;
    return textureSample(t_texture, s_texture, uvs) * in.color;
}
"#;

pub fn pixelated_pipeline(
    res: PipelineResources,
    pixel_ubo: &Buffer,
) -> Result<PipelineContext, String> {
    let pip = gfx::create_render_pipeline(SHADER)
        .with_label("Pixelated Pipeline")
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
        // We will use the 2 group to store our pixel data
        .with_bind_group_layout(
            BindGroupLayout::new()
                .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
        )
        .with_blend_mode(BlendMode::NORMAL)
        .build()?;

    let transform_bind_group = gfx::create_bind_group()
        .with_label("Transform")
        .with_layout(pip.bind_group_layout_ref(0)?)
        .with_uniform(0, res.ubo)
        .build()?;

    let pixel_bind_group = gfx::create_bind_group()
        .with_label("Pixelated")
        .with_layout(pip.bind_group_layout_ref(2)?)
        .with_uniform(0, pixel_ubo)
        .build()?;

    Ok(PipelineContext {
        pipeline: pip,
        groups: (&[
            transform_bind_group,
            res.sprite_bind_group.clone(), // reuse the binding group for texture/samplers
            pixel_bind_group,
        ])
            .as_bind_groups(),
        vertex_offset: 8,
        x_pos: 0,
        y_pos: 1,
        alpha_pos: Some(7),
    })
}

struct State {
    sprite: Sprite,
    pixel_ubo: Buffer,
    pip_id: DrawPipelineId,
}

impl State {
    fn new() -> Result<Self, String> {
        let sprite = draw::create_sprite()
            .from_image(include_bytes!("assets/ferris.png"))
            .build()?;

        // This buffer contains the size of the pixels
        let pixel_ubo = gfx::create_uniform_buffer(&[8.0f32])
            .with_write_flag(true)
            .build()?;

        // register the custom pipeline and get an id for the context in the batching system
        let pip_id = draw::add_pipeline_2d(|res| pixelated_pipeline(res, &pixel_ubo).unwrap());

        Ok(Self {
            sprite,
            pixel_ubo,
            pip_id,
        })
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(|| State::new().unwrap())
        .on_update(update)
        .run()
}

fn update(s: &mut State) {
    let pixel_size = 8.0 + time::elapsed_f32().sin();
    gfx::write_buffer(&s.pixel_ubo)
        .with_data(&[pixel_size])
        .build()
        .unwrap();

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    draw.image(&s.sprite)
        .translate(vec2(200.0, 300.0))
        .anchor(Vec2::splat(0.5));

    draw.image(&s.sprite)
        .translate(vec2(600.0, 300.0))
        .anchor(Vec2::splat(0.5))
        .pipeline(&s.pip_id); // use the custom pipeline

    gfx::render_to_frame(&draw).unwrap();
}
