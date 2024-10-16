use crate::{Draw2D, DrawPipelineId, DrawingInfo, Element2D, PipelineContext, Sprite, Transform2D};
use core::gfx::{
    self, BindGroupLayout, BindingType, BlendMode, Buffer, Color, VertexFormat, VertexLayout,
};
use core::math::{bvec2, Mat3, Vec2};
use macros::Drawable2D;
use num::Zero;

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
    @location(2) frame: vec4<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uvs: vec2<f32>,
    @location(1) frame: vec4<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.frame = model.frame;
    out.color = model.color;
    out.uvs = model.uvs;
    out.position = transform.mvp * vec4(model.position, 0.0, 1.0);
    return out;
}

@group(1) @binding(0)
var t_texture: texture_2d<f32>;
@group(1) @binding(1)
var s_texture: sampler;

// srg to linear
{{SRGB_TO_LINEAR}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let in_color = srgb_to_linear(in.color);
    let coords = in.frame.xy + fract(in.uvs) * in.frame.zw;
    return textureSample(t_texture, s_texture, coords) * in_color;
}
"#;

pub fn create_pattern_2d_pipeline_ctx(ubo_transform: &Buffer) -> Result<PipelineContext, String> {
    let shader = SHADER.replace(
        "{{SRGB_TO_LINEAR}}",
        include_str!("../resources/to_linear.wgsl"),
    );
    let pip = gfx::create_render_pipeline(&shader)
        .with_label("Draw2D pattern default pipeline")
        .with_vertex_layout(
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x2)
                .with_attr(2, VertexFormat::Float32x4)
                .with_attr(3, VertexFormat::Float32x4),
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
        vertex_offset: 12,
        x_pos: 0,
        y_pos: 1,
        alpha_pos: Some(11),
    })
}

#[derive(Drawable2D)]
pub struct Pattern2D {
    sprite: Sprite,
    position: Vec2,
    img_offset: Vec2,
    img_scale: Vec2,
    color: Color,
    alpha: f32,
    size: Option<Vec2>,

    #[pipeline_id]
    pip: DrawPipelineId,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl Pattern2D {
    pub fn new(sprite: &Sprite) -> Self {
        Self {
            sprite: sprite.clone(),
            position: Vec2::ZERO,
            img_offset: Vec2::ZERO,
            img_scale: Vec2::ONE,
            color: Color::WHITE,
            alpha: 1.0,
            size: None,
            pip: DrawPipelineId::Pattern,
            transform: None,
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

    pub fn size(&mut self, size: Vec2) -> &mut Self {
        self.size = Some(size);
        self
    }

    pub fn image_scale(&mut self, scale: Vec2) -> &mut Self {
        self.img_scale = scale;
        self
    }

    pub fn image_offset(&mut self, offset: Vec2) -> &mut Self {
        self.img_offset = offset;
        self
    }
}

impl Element2D for Pattern2D {
    fn process(&self, draw: &mut Draw2D) {
        let c = self.color.with_alpha(self.color.a * self.alpha);
        let size = self.size.unwrap_or(self.sprite.size());
        let Vec2 { x: x1, y: y1 } = self.position;
        let Vec2 { x: x2, y: y2 } = self.position + size;

        let frame = self.sprite.frame();
        let scaled_size = self.img_scale * frame.size;
        debug_assert!(
            !scaled_size.x.is_zero() && !scaled_size.y.is_zero(),
            "Pattern scaled size should not be 0"
        );
        let offset = ((self.img_offset * self.img_scale) / scaled_size).fract();
        let uv_size = size / scaled_size;

        let Vec2 { x: u1, y: v1 } = offset;
        let Vec2 { x: u2, y: v2 } = uv_size + offset;

        let base_size = self.sprite.texture().size();
        let Vec2 { x: fx, y: fy } = frame.origin / base_size;
        let Vec2 { x: fw, y: fh } = frame.size / base_size;

        #[rustfmt::skip]
        let mut vertices = [
            x1, y1, u1, v1, fx, fy, fw, fh, c.r, c.g, c.b, c.a,
            x2, y1, u2, v1, fx, fy, fw, fh, c.r, c.g, c.b, c.a,
            x1, y2, u1, v2, fx, fy, fw, fh, c.r, c.g, c.b, c.a,
            x2, y2, u2, v2, fx, fy, fw, fh, c.r, c.g, c.b, c.a,
        ];

        let indices = [0, 1, 2, 2, 1, 3];

        let matrix = self
            .transform
            .map_or(Mat3::IDENTITY, |mut t| t.set_size(size).updated_mat3());

        draw.add_to_batch(DrawingInfo {
            pipeline: self.pip,
            vertices: &mut vertices,
            indices: &indices,
            transform: matrix,
            sprite: Some(&self.sprite),
        })
    }
}
