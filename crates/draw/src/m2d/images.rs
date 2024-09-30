use crate::{
    AsBindGroups, Draw2D, DrawPipelineId, DrawingInfo, Element2D, PipelineContext, Sprite,
    Transform2D,
};
use core::gfx::{
    self, BindGroupLayout, BindingType, BlendMode, Buffer, Color, VertexFormat, VertexLayout,
};
use core::math::{bvec2, Mat3, Rect, Vec2};
use internment::Intern;
use macros::Transform2D;

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
        groups: (&[bind_group]).as_bind_groups(),
        vertex_offset: 8,
        x_pos: 0,
        y_pos: 1,
        alpha_pos: Some(7),
    })
}

#[derive(Transform2D)]
pub struct Image2D {
    sprite: Sprite,
    position: Vec2,
    color: Color,
    alpha: f32,
    size: Option<Vec2>,
    crop: Option<Rect>,

    pip: DrawPipelineId,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl Image2D {
    pub fn new(sprite: &Sprite) -> Self {
        Self {
            sprite: sprite.clone(),
            position: Vec2::ZERO,
            color: Color::WHITE,
            alpha: 1.0,
            crop: None,
            size: None,
            pip: DrawPipelineId::Images,
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

    pub fn crop(&mut self, origin: Vec2, size: Vec2) -> &mut Self {
        self.crop = Some(Rect::new(origin, size));
        self.size(size)
    }

    pub fn pipeline(&mut self, pip: &DrawPipelineId) -> &mut Self {
        self.pip = *pip;
        self
    }
}

impl Element2D for Image2D {
    fn process(&self, draw: &mut Draw2D) {
        let c = self.color.with_alpha(self.color.a * self.alpha);
        let size = self.size.unwrap_or(self.sprite.size());
        let Vec2 { x: x1, y: y1 } = self.position;
        let Vec2 { x: x2, y: y2 } = self.position + size;

        let frame = self.sprite.frame();
        let Rect {
            origin: Vec2 { x: sx, y: sy },
            size: Vec2 { x: sw, y: sh },
        } = self.crop.map_or(frame, |mut r| {
            r.origin += frame.origin;
            r
        });

        let (u1, v1, u2, v2) = {
            let Vec2 { x: tw, y: th } = self.sprite.texture().size();
            let u1 = sx / tw;
            let v1 = sy / th;
            let u2 = (sx + sw) / tw;
            let v2 = (sy + sh) / th;

            (u1, v1, u2, v2)
        };

        #[rustfmt::skip]
        let mut vertices = [
            x1, y1, u1, v1, c.r, c.g, c.b, c.a,
            x2, y1, u2, v1, c.r, c.g, c.b, c.a,
            x1, y2, u1, v2, c.r, c.g, c.b, c.a,
            x2, y2, u2, v2, c.r, c.g, c.b, c.a,
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
