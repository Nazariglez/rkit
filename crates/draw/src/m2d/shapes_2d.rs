use super::{Draw2D, DrawPipeline, DrawingInfo, Element2D, PipelineContext};
use core::gfx::{self, BindGroupLayout, BindingType, Buffer, Color, VertexFormat, VertexLayout};
use core::math::{Mat3, Vec2};
use smallvec::SmallVec;

const VERTICES_OFFSET: usize = 6; // pos(f32x2) + color(f32x4)

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

// NOTE: The idea of the hint is to avoid allocations on the heap for most common shapes
// there are going to be case of complex shapes, or with a lot of lines, or with
// rounded corners where is going to need more space, so is going to be heap allocated
// but the alternative is to alloc everything on the heap, and I am trying to avoid it
// because this will be most likely called once per frame.
//
// Usually we will use for triangulating something along the lines of if n>=3 -> (n-2),
// however, we need to think about rounded corners, giving them some margin, so I am
// trying with 5 triangles per point, and we can adjust in the future
// pub struct Shape<const VERTICES_HINT: usize, const INDICES_HINT: usize> {
//     vertices: SmallVec<f32, VERTICES_HINT>,
//     indices: SmallVec<u32, INDICES_HINT>,
// }

pub struct Triangle {
    points: [Vec2; 3],
    color: Color,
    alpha: f32,
    transform: Mat3,
    // cached: Option<Shape<6, 3>>
}

impl Triangle {
    pub fn new(p1: Vec2, p2: Vec2, p3: Vec2) -> Self {
        Self {
            points: [p1, p2, p3],
            color: Color::WHITE,
            alpha: 1.0,
            transform: Mat3::IDENTITY,
            // cached: None,
        }
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }
}

impl Element2D for Triangle {
    fn process(&self, draw: &mut Draw2D) {
        // compute matrix

        let [a, b, c] = self.points;
        let color = self.color;
        let alpha = self.alpha;

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
            offset: VERTICES_OFFSET,
            transform: self.transform,
            texture: None,
        })
    }
}
