mod cirlce;
mod ellipse;
mod line;
mod path;
mod polygon;
mod rectangle;
mod star;
mod triangle;

pub use cirlce::*;
pub use ellipse::*;
pub use line::*;
pub use path::*;
pub use polygon::*;
pub use rectangle::*;
pub use star::*;
pub use triangle::*;

use super::PipelineContext;
use core::gfx::{
    self, BindGroupLayout, BindingType, BlendMode, Buffer, VertexFormat, VertexLayout,
};
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

// srgb_to_linear
{{SRGB_TO_LINEAR}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return srgb_to_linear(in.color);
}
"#;

pub fn create_shapes_2d_pipeline_ctx(ubo_transform: &Buffer) -> Result<PipelineContext, String> {
    let shader = SHADER.replace(
        "{{SRGB_TO_LINEAR}}",
        include_str!("../../resources/to_linear.wgsl"),
    );
    let pip = gfx::create_render_pipeline(&shader)
        .with_vertex_layout(
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x4),
        )
        .with_bind_group_layout(
            BindGroupLayout::new().with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
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
        vertex_offset: 6,
        x_pos: 0,
        y_pos: 1,
        alpha_pos: Some(5),
    })
}

pub(crate) fn size_from_vertices(vertices: &[f32]) -> Vec2 {
    const OFFSET: usize = 6;

    let mut min = Vec2::MAX;
    let mut max = Vec2::MIN;

    vertices.chunks_exact(OFFSET).for_each(|chunks| {
        let (x, y) = (chunks[0], chunks[1]);
        if x < min.x {
            min.x = x;
        }
        if y < min.y {
            min.y = y;
        }
        if x > max.x {
            max.x = x;
        }
        if y > max.y {
            max.y = y;
        }
    });

    max - min
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_from_vertices() {
        #[rustfmt::skip]
        let vertices: [f32; 18] = [
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            2.0, 1.0, 0.0, 0.0, 0.0, 0.0,
            1.0, 3.0, 0.0, 0.0, 0.0, 0.0,
        ];

        let expected_size = Vec2::new(2.0, 3.0);
        let result = size_from_vertices(&vertices);
        assert_eq!(result, expected_size);
    }

    #[test]
    fn test_size_from_vertices_negative() {
        #[rustfmt::skip]
        let vertices: [f32; 18] = [
            -1.0, -2.0, 0.0, 0.0, 0.0, 0.0,
            3.0, 4.0, 0.0, 0.0, 0.0, 0.0,
            -2.0, 1.0, 0.0, 0.0, 0.0, 0.0,
        ];

        let expected_size = Vec2::new(5.0, 6.0);
        let result = size_from_vertices(&vertices);
        assert_eq!(result, expected_size);
    }
}
