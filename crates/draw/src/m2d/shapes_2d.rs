use super::{Draw2D, DrawPipeline, DrawingInfo, Element2D, PipelineContext};
use core::gfx::{self, BindGroupLayout, BindingType, Buffer, Color, VertexFormat, VertexLayout};
use core::math::Vec2;

// language=wgsl
const SHADER: &str = r#"
struct Transform {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> transform: Transform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.position = transform.mvp * vec4(model.position, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

pub fn create_shapes_2d_pipeline_ctx(ubo_transform: &Buffer) -> Result<PipelineContext, String> {
    let pip = gfx::create_render_pipeline(SHADER)
        .with_vertex_layout(
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x4),
        )
        .with_bind_group_layout(
            BindGroupLayout::new().with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
        )
        .build()?;

    let bind_group = gfx::create_bind_group()
        .with_layout(pip.bind_group_layout_id(0)?)
        .with_uniform(0, &ubo_transform)
        .build()?;

    Ok(PipelineContext {
        pipeline: pip,
        groups: (&[bind_group] as &[_]).try_into().unwrap(),
    })
}

pub struct Triangle {
    points: [Vec2; 3],
    color: Color,
}

impl Triangle {
    pub fn new(p1: Vec2, p2: Vec2, p3: Vec2) -> Self {
        Self {
            points: [p1, p2, p3],
            color: Color::WHITE,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl Element2D for Triangle {
    fn process(&self, draw: &mut Draw2D) {
        let alpha = draw.alpha();

        // compute matrix

        let [a, b, c] = self.points;
        let color = self.color;

        #[rustfmt::skip]
        let vertices = [
            a.x, a.y, color.r, color.g, color.b, color.a * alpha,
            b.x, b.y, color.r, color.g, color.b, color.a * alpha,
            c.x, c.y, color.r, color.g, color.b, color.a * alpha,
        ];

        let indices = [0, 1, 2];

        draw.add_to_batch(DrawingInfo {
            pipeline: DrawPipeline::Shapes,
            vertices: &vertices,
            indices: &indices,
        })
    }
}
