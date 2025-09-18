use crate::text::{AtlasType, Font, HAlign, TextInfo, get_mut_text_system};
use crate::{Draw2D, DrawPipelineId, DrawingInfo, Element2D, PipelineContext, Transform2D};
use corelib::gfx::{
    self, BindGroupLayout, BindingType, BlendMode, Buffer, Color, VertexFormat, VertexLayout,
};
use corelib::math::{Mat3, Rect, Vec2, bvec2};
use macros::Drawable2D;
use std::cell::RefCell;

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
use corelib::app::is_window_pixelated;

thread_local! {
    static TEMP_VERTICES: RefCell<Vec<f32>> = const { RefCell::new(vec![]) };
    static TEMP_INDICES: RefCell<Vec<u32>> = const { RefCell::new(vec![]) };
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
    @location(2) tex: f32,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uvs: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) tex: f32,
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
var s_linear: sampler;
@group(1) @binding(1)
var s_nearest: sampler;
@group(1) @binding(2)
var t_mask: texture_2d<f32>;
@group(1) @binding(3)
var t_color: texture_2d<f32>;

// srg to linear
{{SRGB_TO_LINEAR}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let in_color = srgb_to_linear(in.color);

    // naga translation to webgl does not support using multiple samples per texture but webgpu does
    {{SELECT_TEXTURE_AND_SAMPLER}}

    // emojis
    let color_sample = textureSampleLevel(t_color, s_linear, in.uvs, 0.0);
    return color_sample * in_color;
}
"#;

#[cfg(any(not(target_arch = "wasm32"), not(feature = "webgl")))]
const SELECT_TEXTURE_SAMPLER: &str = r#"
    // linear
    if (in.tex == 0.0) {
        let mask_sample = textureSampleLevel(t_mask, s_linear, in.uvs, 0.0);
        return vec4(in_color.rgb, mask_sample.r * in_color.a);
    }

    // nearest
    if (in.tex == 1.0) {
        let mask_sample = textureSampleLevel(t_mask, s_nearest, in.uvs, 0.0);
        return vec4(in_color.rgb, mask_sample.r * in_color.a);
    }
"#;

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
const SELECT_TEXTURE_SAMPLER_LINEAR: &str = r#"
    // linear
    if (in.tex != 2.0) {
        let mask_sample = textureSampleLevel(t_mask, s_linear, in.uvs, 0.0);
        return vec4(in_color.rgb, mask_sample.r * in_color.a);
    }
"#;

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
const SELECT_TEXTURE_SAMPLER_NEAREST: &str = r#"
    // nearest
    if (in.tex != 2.0) {
        let mask_sample = textureSampleLevel(t_mask, s_nearest, in.uvs, 0.0);
        return vec4(in_color.rgb, mask_sample.r * in_color.a);
    }
"#;

// IMPORTANT NOTE: WebGL shader translation does not support multiple shader per texture
// to make this work, webgpu targets will select the sampler based on the font but
// webgl will select only one sampler based on the window "pixelated" flag. This means that
// if the window is defined as pixelated all font will use a NEAREST sampler while targeting WebGL
fn select_texture_sampler() -> &'static str {
    #[cfg(any(not(target_arch = "wasm32"), not(feature = "webgl")))]
    {
        #[allow(clippy::needless_return)]
        return SELECT_TEXTURE_SAMPLER;
    }

    #[cfg(all(target_arch = "wasm32", feature = "webgl"))]
    {
        if is_window_pixelated() {
            SELECT_TEXTURE_SAMPLER_NEAREST
        } else {
            SELECT_TEXTURE_SAMPLER_LINEAR
        }
    }
}

// TODO: alternatively we can create a new pipeline for WebGL using only one sampler and then
// we swap the binding group depending if the font is pixelated or not. This will match the WebGPU
// behavior where the same app can have pixelated and linear fonts, the downside is that this
// we'll need to swap the batches if we're swapping between pixelated or linear fonts incurring in more
// draw calls. Which is probably worth it if we want better webgl suppor.
pub fn create_text_2d_pipeline_ctx(ubo_transform: &Buffer) -> Result<PipelineContext, String> {
    let shader = SHADER
        .replace(
            "{{SRGB_TO_LINEAR}}",
            include_str!("../resources/to_linear.wgsl"),
        )
        .replace("{{SELECT_TEXTURE_AND_SAMPLER}}", select_texture_sampler());

    let pip = gfx::create_render_pipeline(&shader)
        .with_label("Draw2D text default pipeline")
        .with_vertex_layout(
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x2)
                .with_attr(2, VertexFormat::Float32)
                .with_attr(3, VertexFormat::Float32x4),
        )
        .with_bind_group_layout(
            BindGroupLayout::new().with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
        )
        .with_bind_group_layout(
            BindGroupLayout::new()
                .with_entry(BindingType::sampler(0).with_fragment_visibility(true))
                .with_entry(BindingType::sampler(1).with_fragment_visibility(true))
                .with_entry(BindingType::texture(2).with_fragment_visibility(true))
                .with_entry(BindingType::texture(3).with_fragment_visibility(true)),
        )
        .with_blend_mode(BlendMode::NORMAL)
        .build()?;

    let bind_group = gfx::create_bind_group()
        .with_label("Draw2D text BindGroup")
        .with_layout(pip.bind_group_layout_ref(0)?)
        .with_uniform(0, ubo_transform)
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

#[derive(Drawable2D)]
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
    res: f32,

    #[pipeline_id]
    pip: DrawPipelineId,

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
            res: 1.0,

            pip: DrawPipelineId::Text,
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

    pub fn resolution(&mut self, res: f32) -> &mut Self {
        self.res = res;
        self
    }
}

impl Element2D for Text2D<'_> {
    fn process(&self, draw: &mut Draw2D) {
        let info = TextInfo {
            pos: self.position,
            font: self.font,
            text: self.text,
            wrap_width: self.max_width,
            font_size: self.size,
            line_height: self.line_height,
            resolution: self.res,
            h_align: self.h_align,
        };

        let c = self.color.with_alpha(self.color.a * self.alpha);

        TEMP_VERTICES.with_borrow_mut(|temp_vertices| {
            TEMP_INDICES.with_borrow_mut(|temp_indices| {
                temp_vertices.clear();
                temp_indices.clear();

                let block_size = {
                    let mut sys = get_mut_text_system();
                    let block = sys.prepare_text(&info, false).unwrap();
                    if block.data.is_empty() {
                        return;
                    }

                    block.data.iter().enumerate().for_each(|(i, data)| {
                        let Vec2 { x: x1, y: y1 } = data.xy;
                        let Vec2 { x: x2, y: y2 } = data.xy + data.size;
                        let Vec2 { x: u1, y: v1 } = data.uvs1;
                        let Vec2 { x: u2, y: v2 } = data.uvs2;
                        let t = match data.typ {
                            AtlasType::Mask if data.pixelated => 1.0,
                            AtlasType::Mask => 0.0,
                            _ => 2.0,
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

                let (mut matrix, pos, anchor) =
                    self.transform
                        .map_or((Mat3::IDENTITY, Vec2::ZERO, Vec2::ZERO), |mut t| {
                            t.set_size(block_size);
                            (t.updated_mat3(), t.position(), t.anchor())
                        });

                if self.res > 1.0 {
                    matrix *= Mat3::from_scale(Vec2::splat(1.0 / self.res));
                }

                draw.add_to_batch(DrawingInfo {
                    pipeline: self.pip,
                    vertices: temp_vertices,
                    indices: temp_indices,
                    transform: matrix,
                    sprite: None,
                });

                let origin = self.position + pos - anchor * block_size;
                draw.last_text_bounds = Rect::new(origin, block_size);
            });
        });
    }
}
