use crate::{Draw2D, DrawPipeline, DrawingInfo, Element2D, PipelineContext, Sprite};
use core::gfx::{
    self, BindGroupLayout, BindingType, BlendMode, Buffer, Color, VertexFormat, VertexLayout,
};
use core::math::{Mat3, UVec2, Vec2};

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
    return textureSample(t_texture, s_texture, in.uvs) * in.color;
}
"#;

pub fn create_images_2d_pipeline_ctx(ubo_transform: &Buffer) -> Result<PipelineContext, String> {
    let pip = gfx::create_render_pipeline(SHADER)
        .with_label("Draw2D images default pipeline")
        .with_vertex_layout(
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x2)
                .with_attr(2, VertexFormat::Float32x4),
        )
        .with_bind_group_layout(
            BindGroupLayout::new().with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
        )
        .with_bind_group_layout(
            BindGroupLayout::new()
                .with_entry(BindingType::texture(0).with_fragment_visibility(true))
                .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
        )
        .with_blend_mode(BlendMode::NORMAL)
        .build()?;

    let bind_group = gfx::create_bind_group()
        .with_layout(pip.bind_group_layout_ref(0)?)
        .with_uniform(0, &ubo_transform)
        .build()?;

    Ok(PipelineContext {
        pipeline: pip,
        groups: (&[bind_group] as &[_]).try_into().unwrap(),
        vertex_offset: 8,
        x_pos: 0,
        y_pos: 1,
        alpha_pos: Some(5),
    })
}

pub struct Image {
    sprite: Sprite,
    position: Vec2,
    color: Color,
    alpha: f32,
    transform: Mat3,
}

impl Image {
    pub fn new(sprite: &Sprite) -> Self {
        Self {
            sprite: sprite.clone(),
            position: Vec2::ZERO,
            color: Color::WHITE,
            alpha: 1.0,
            transform: Mat3::IDENTITY,
        }
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }

    pub fn alpha(&mut self, alpha: f32) -> &mut Self {
        self.alpha = alpha;
        self
    }

    pub fn position(&mut self, pos: Vec2) -> &mut Self {
        self.position = pos;
        self
    }
}

impl Element2D for Image {
    fn process(&self, draw: &mut Draw2D) {
        let c = self.color.with_alpha(self.color.a * self.alpha);
        let Vec2 { x: x1, y: y1 } = self.position;
        let UVec2 { x: x2, y: y2 } = self.sprite.size();
        let (x2, y2) = (x1 + x2 as f32, y1 + y2 as f32);

        let (u1, v1, u2, v2) = (0.0, 0.0, 1.0, 1.0);

        #[rustfmt::skip]
        let mut vertices = [
            x1, y1, u1, v1, c.r, c.g, c.b, c.a,
            x2, y1, u2, v1, c.r, c.g, c.b, c.a,
            x1, y2, u1, v2, c.r, c.g, c.b, c.a,
            x2, y2, u2, v2, c.r, c.g, c.b, c.a,
        ];

        let indices = [0, 1, 2, 2, 1, 3];

        draw.add_to_batch(DrawingInfo {
            pipeline: DrawPipeline::Images,
            vertices: &mut vertices,
            indices: &indices,
            transform: self.transform,
            sprite: Some(&self.sprite),
        })
    }
}
