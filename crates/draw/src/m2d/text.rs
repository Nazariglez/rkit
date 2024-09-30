use crate::text::{get_mut_text_system, AtlasType, Font, HAlign, TextInfo};
use crate::{Draw2D, DrawPipelineId, DrawingInfo, Element2D, PipelineContext, Transform2D};
use core::gfx::{
    self, BindGroupLayout, BindingType, BlendMode, Buffer, Color, VertexFormat, VertexLayout,
};
use core::math::{bvec2, Mat3, Rect, Vec2};
use macros::Transform2D;
use std::cell::RefCell;

thread_local! {
    static TEMP_VERTICES: RefCell<Vec<f32>> = RefCell::new(vec![]);
    static TEMP_INDICES: RefCell<Vec<u32>> = RefCell::new(vec![]);
}

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
    @location(2) tex: u32,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uvs: vec2<f32>,
    @location(1) @interpolate(flat) tex: u32,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex = model.tex;
    out.color = model.color;
    out.uvs = model.uvs;
    out.position = transform.mvp * vec4(model.position, 0.0, 1.0);
    return out;
}

@group(1) @binding(0)
var s_texture: sampler;
@group(1) @binding(1)
var t_mask: texture_2d<f32>;
@group(1) @binding(2)
var t_color: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.tex == 0u) {
        let mask_sample = textureSampleLevel(t_mask, s_texture, in.uvs, 0.0);
        var color: vec4<f32> = vec4(in.color.rgb, mask_sample.r * in.color.a);
        if color.a <= 0.0 {
            discard;
        }
        return color;
    } else {
        let color_sample = textureSampleLevel(t_color, s_texture, in.uvs, 0.0);
        return color_sample * in.color;
    }
}
"#;

pub fn create_text_2d_pipeline_ctx(ubo_transform: &Buffer) -> Result<PipelineContext, String> {
    let pip = gfx::create_render_pipeline(SHADER)
        .with_label("Draw2D text default pipeline")
        .with_vertex_layout(
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x2)
                .with_attr(2, VertexFormat::UInt32)
                .with_attr(3, VertexFormat::Float32x4),
        )
        .with_bind_group_layout(
            BindGroupLayout::new().with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
        )
        .with_bind_group_layout(
            BindGroupLayout::new()
                .with_entry(BindingType::sampler(0).with_fragment_visibility(true))
                .with_entry(BindingType::texture(1).with_fragment_visibility(true))
                .with_entry(BindingType::texture(2).with_fragment_visibility(true)),
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
        vertex_offset: 9,
        x_pos: 0,
        y_pos: 1,
        alpha_pos: Some(8),
    })
}

#[derive(Transform2D)]
pub struct Text2D<'a> {
    text: &'a str,
    font: Option<&'a Font>,
    position: Vec2,
    color: Color,
    alpha: f32,
    size: f32,
    line_height: Option<f32>,
    max_width: Option<f32>,
    h_align: HAlign,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl<'a> Text2D<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            font: None,
            position: Vec2::ZERO,
            color: Color::WHITE,
            alpha: 1.0,
            size: 14.0,
            line_height: None,
            max_width: None,
            h_align: HAlign::default(),

            transform: None,
        }
    }

    pub fn font(&mut self, font: &'a Font) -> &mut Self {
        self.font = Some(font);
        self
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

    pub fn size(&mut self, size: f32) -> &mut Self {
        self.size = size;
        self
    }

    pub fn line_height(&mut self, height: f32) -> &mut Self {
        self.line_height = Some(height);
        self
    }

    pub fn max_width(&mut self, width: f32) -> &mut Self {
        self.max_width = Some(width);
        self
    }

    pub fn h_align_left(&mut self) -> &mut Self {
        self.h_align = HAlign::Left;
        self
    }

    pub fn h_align_center(&mut self) -> &mut Self {
        self.h_align = HAlign::Center;
        self
    }

    pub fn h_align_right(&mut self) -> &mut Self {
        self.h_align = HAlign::Right;
        self
    }
}

impl<'a> Element2D for Text2D<'a> {
    fn process(&self, draw: &mut Draw2D) {
        let info = TextInfo {
            pos: self.position,
            font: self.font.clone(),
            text: self.text,
            wrap_width: self.max_width,
            font_size: self.size,
            line_height: self.line_height,
            scale: 1.0,
            h_align: self.h_align,
        };

        let c = self.color;

        TEMP_VERTICES.with_borrow_mut(|temp_vertices| {
            TEMP_INDICES.with_borrow_mut(|temp_indices| {
                temp_vertices.clear();
                temp_indices.clear();

                let block_size = {
                    let mut sys = get_mut_text_system();
                    let block = sys.prepare_text(&info).unwrap();
                    if block.data.is_empty() {
                        return;
                    }

                    block.data.iter().enumerate().for_each(|(i, data)| {
                        let Vec2 { x: x1, y: y1 } = data.xy;
                        let Vec2 { x: x2, y: y2 } = data.xy + data.size;
                        let Vec2 { x: u1, y: v1 } = data.uvs1;
                        let Vec2 { x: u2, y: v2 } = data.uvs2;
                        let t: f32 = if matches!(data.typ, AtlasType::Mask) {
                            0.0
                        } else {
                            1.0
                        };

                        #[rustfmt::skip]
                        let vertices = [
                            x1, y1, u1, v1, t, c.r, c.g, c.b, c.a,
                            x2, y1, u2, v1, t, c.r, c.g, c.b, c.a,
                            x1, y2, u1, v2, t, c.r, c.g, c.b, c.a,
                            x2, y2, u2, v2, t, c.r, c.g, c.b, c.a,
                        ];

                        let n = (i * 4) as u32;

                        #[rustfmt::skip]
                        let indices = [
                            n,     n + 1,   n + 2,
                            n + 2, n + 1,   n + 3
                        ];

                        temp_vertices.extend_from_slice(vertices.as_slice());
                        temp_indices.extend_from_slice(indices.as_slice());
                    });

                    block.size
                };

                let (matrix, pos, anchor) =
                    self.transform
                        .map_or((Mat3::IDENTITY, Vec2::ZERO, Vec2::ZERO), |mut t| {
                            t.set_size(block_size);
                            (t.updated_mat3(), t.position(), t.anchor())
                        });

                draw.add_to_batch(DrawingInfo {
                    pipeline: DrawPipelineId::Text,
                    vertices: temp_vertices,
                    indices: temp_indices,
                    transform: matrix,
                    sprite: None,
                });

                let origin = pos - anchor * block_size;
                draw.last_text_bounds = Rect::new(origin, block_size);
            });
        });
    }
}
