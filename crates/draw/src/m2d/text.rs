use crate::text::{
    Font, HAlign, QuadData, RichTextLayout, TextInfo, TextSystem, get_mut_text_system,
};
use crate::{Draw2D, DrawPipelineId, DrawingInfo, Element2D, PipelineContext, Transform2D};
use corelib::gfx::{
    self, BindGroupLayout, BindingType, BlendMode, Buffer, Color, VertexFormat, VertexLayout,
};
use corelib::math::{IntoVec2, Rect, Vec2, bvec2, vec2, vec3};
use macros::Drawable2D;
use std::{borrow::Cow, cell::RefCell};

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
use corelib::app::is_window_pixelated;

thread_local! {
    static TEMP_VERTICES: RefCell<Vec<f32>> = const { RefCell::new(vec![]) };
    static TEMP_INDICES: RefCell<Vec<u32>> = const { RefCell::new(vec![]) };
    static TEMP_QUADS: RefCell<Vec<QuadData>> = const { RefCell::new(vec![]) };
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
var t_rgba_linear: texture_2d<f32>;
@group(1) @binding(4)
var t_rgba_nearest: texture_2d<f32>;

// srg to linear
{{SRGB_TO_LINEAR}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let in_color = srgb_to_linear(in.color);

    {{SELECT_TEXTURE_AND_SAMPLER}}
}
"#;

#[cfg(any(not(target_arch = "wasm32"), not(feature = "webgl")))]
const SELECT_TEXTURE_SAMPLER: &str = r#"
    if (in.tex == 0.0) {
        let mask = textureSampleLevel(t_mask, s_linear, in.uvs, 0.0);
        return vec4(in_color.rgb, mask.r * in_color.a);
    }
    if (in.tex == 1.0) {
        let mask = textureSampleLevel(t_mask, s_nearest, in.uvs, 0.0);
        return vec4(in_color.rgb, mask.r * in_color.a);
    }
    if (in.tex == 2.0) {
        return textureSampleLevel(t_rgba_linear, s_linear, in.uvs, 0.0) * in_color;
    }
    if (in.tex == 3.0) {
        return textureSampleLevel(t_rgba_nearest, s_nearest, in.uvs, 0.0) * in_color;
    }
    if (in.tex == 4.0) {
        let rgba = textureSampleLevel(t_rgba_linear, s_linear, in.uvs, 0.0);
        return vec4(in_color.rgb, rgba.a * in_color.a);
    }
    let rgba = textureSampleLevel(t_rgba_nearest, s_nearest, in.uvs, 0.0);
    return vec4(in_color.rgb, rgba.a * in_color.a);
"#;

#[cfg(all(target_arch = "wasm32", feature = "webgl"))]
const SELECT_TEXTURE_SAMPLER_WEBGL: &str = r#"
    if (in.tex == 0.0 || in.tex == 1.0) {
        let mask = textureSampleLevel(t_mask, {{MASK_SAMPLER}}, in.uvs, 0.0);
        return vec4(in_color.rgb, mask.r * in_color.a);
    }
    if (in.tex == 2.0) {
        return textureSampleLevel(t_rgba_linear, s_linear, in.uvs, 0.0) * in_color;
    }
    if (in.tex == 3.0) {
        return textureSampleLevel(t_rgba_nearest, s_nearest, in.uvs, 0.0) * in_color;
    }
    if (in.tex == 4.0) {
        let rgba = textureSampleLevel(t_rgba_linear, s_linear, in.uvs, 0.0);
        return vec4(in_color.rgb, rgba.a * in_color.a);
    }
    let rgba = textureSampleLevel(t_rgba_nearest, s_nearest, in.uvs, 0.0);
    return vec4(in_color.rgb, rgba.a * in_color.a);
"#;

fn select_texture_sampler() -> Cow<'static, str> {
    #[cfg(any(not(target_arch = "wasm32"), not(feature = "webgl")))]
    {
        Cow::Borrowed(SELECT_TEXTURE_SAMPLER)
    }

    #[cfg(all(target_arch = "wasm32", feature = "webgl"))]
    {
        let sampler = if is_window_pixelated() {
            "s_nearest"
        } else {
            "s_linear"
        };
        Cow::Owned(SELECT_TEXTURE_SAMPLER_WEBGL.replace("{{MASK_SAMPLER}}", sampler))
    }
}

pub fn create_text_2d_pipeline_ctx(ubo_transform: &Buffer) -> Result<PipelineContext, String> {
    let shader = SHADER
        .replace(
            "{{SRGB_TO_LINEAR}}",
            include_str!("../resources/to_linear.wgsl"),
        )
        .replace("{{SELECT_TEXTURE_AND_SAMPLER}}", &select_texture_sampler());

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
                .with_entry(BindingType::texture(3).with_fragment_visibility(true))
                .with_entry(BindingType::texture(4).with_fragment_visibility(true)),
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
    resolution_override: Option<f32>,
    shadow_color: Color,
    shadow_offset: Option<Vec2>,
    color_tags: bool,
    outline_color: Color,
    outline_width: u16,

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
            resolution_override: None,
            shadow_color: Color::BLACK,
            shadow_offset: None,
            color_tags: false,
            outline_color: Color::BLACK,
            outline_width: 0,

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

    pub fn resolution(&mut self, resolution: f32) -> &mut Self {
        self.resolution_override = Some(resolution);
        self
    }

    pub fn shadow_color(&mut self, color: Color) -> &mut Self {
        self.shadow_color = color;
        self
    }

    pub fn shadow_offset(&mut self, offset: impl IntoVec2) -> &mut Self {
        self.shadow_offset = Some(offset.into_vec2());
        self
    }

    pub fn color_tags(&mut self) -> &mut Self {
        self.color_tags = true;
        self
    }

    pub fn outline(&mut self, color: Color, width: u16) -> &mut Self {
        self.outline_color = color;
        self.outline_width = width;
        self
    }
}

enum TextPass {
    ShadowOutline,
    ShadowFill,
    Outline,
    Fill,
}

struct TextBatchPass {
    position: Vec2,
    alpha: f32,
    solid_color: Option<Color>,
    outline: bool,
    rgba_as_mask: bool,
    transform: corelib::math::Mat3,
    pip: DrawPipelineId,
}

impl Element2D for Text2D<'_> {
    fn process(&self, draw: &mut Draw2D) {
        let info = TextInfo {
            font: self.font,
            text: self.text,
            wrap_width: self.max_width,
            font_size: self.size,
            line_height: self.line_height,
            h_align: self.h_align,
            color_tags: self.color_tags,
            default_color: self.color,
            outline_width: self.outline_width,
            strict_metrics: false,
        };
        TEMP_QUADS.with_borrow_mut(|quads| {
            let mut system = get_mut_text_system();
            let mut layout = system.take_layout();
            if let Err(error) = system.layout_text(&info, &mut layout) {
                system.recycle_layout(layout);
                drop(system);
                draw.record_error(error);
                return;
            }

            let size = layout.size;
            let transform = finalized_text_transform(size, self.transform);
            let resolution = match self.resolution_override {
                Some(resolution) => resolution,
                None => match draw.target_resolution(
                    transform * corelib::math::Mat3::from_translation(self.position),
                ) {
                    Ok(resolution) => TextSystem::automatic_resolution(resolution),
                    Err(error) => {
                        system.recycle_layout(layout);
                        drop(system);
                        draw.record_error(error);
                        return;
                    }
                },
            };
            let prepared = system.prepare_layout(&layout, resolution, quads);
            system.recycle_layout(layout);
            drop(system);
            if let Err(error) = prepared {
                draw.record_error(error);
                return;
            }

            set_text_bounds(draw, self.position, size, transform);
            if quads.is_empty() {
                return;
            }
            let outlined = self.outline_width > 0;
            if self.shadow_offset.is_some() {
                if outlined {
                    add_text_to_batch(self, quads, transform, TextPass::ShadowOutline, draw);
                }
                add_text_to_batch(self, quads, transform, TextPass::ShadowFill, draw);
            }
            if outlined {
                add_text_to_batch(self, quads, transform, TextPass::Outline, draw);
            }
            add_text_to_batch(self, quads, transform, TextPass::Fill, draw);
        });
    }
}

#[derive(Drawable2D)]
pub struct RichText2D<'a> {
    layout: &'a RichTextLayout,
    position: Vec2,
    alpha: f32,
    resolution_override: Option<f32>,
    shadow_color: Color,
    shadow_offset: Option<Vec2>,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl<'a> RichText2D<'a> {
    pub(crate) fn new(layout: &'a RichTextLayout) -> Self {
        Self {
            layout,
            position: Vec2::ZERO,
            alpha: 1.0,
            resolution_override: None,
            shadow_color: Color::BLACK,
            shadow_offset: None,
            transform: None,
        }
    }

    pub fn position(&mut self, position: Vec2) -> &mut Self {
        self.position = position;
        self
    }

    pub fn alpha(&mut self, alpha: f32) -> &mut Self {
        self.alpha = alpha;
        self
    }

    pub fn resolution(&mut self, resolution: f32) -> &mut Self {
        self.resolution_override = Some(resolution);
        self
    }

    pub fn shadow_color(&mut self, color: Color) -> &mut Self {
        self.shadow_color = color;
        self
    }

    pub fn shadow_offset(&mut self, offset: impl IntoVec2) -> &mut Self {
        self.shadow_offset = Some(offset.into_vec2());
        self
    }
}

impl Element2D for RichText2D<'_> {
    fn process(&self, draw: &mut Draw2D) {
        let size = self.layout.size();
        let transform = finalized_text_transform(size, self.transform);
        let fixed_resolution = self.resolution_override.or(self.layout.resolution);
        let resolution = match fixed_resolution {
            Some(resolution) => resolution,
            None => match draw
                .target_resolution(transform * corelib::math::Mat3::from_translation(self.position))
            {
                Ok(resolution) => TextSystem::automatic_resolution(resolution),
                Err(error) => {
                    draw.record_error(error);
                    return;
                }
            },
        };

        TEMP_QUADS.with_borrow_mut(|quads| {
            let mut system = get_mut_text_system();
            let prepared = system.prepare_layout(&self.layout.layout, resolution, quads);
            drop(system);
            if let Err(error) = prepared {
                draw.record_error(error);
                return;
            }

            set_text_bounds(draw, self.position, size, transform);
            if let Some(offset) = self.shadow_offset {
                add_quads_to_batch(
                    quads,
                    TextBatchPass {
                        position: self.position + offset,
                        alpha: self.alpha,
                        solid_color: Some(self.shadow_color),
                        outline: false,
                        rgba_as_mask: true,
                        transform,
                        pip: DrawPipelineId::Text,
                    },
                    draw,
                );
            }
            add_quads_to_batch(
                quads,
                TextBatchPass {
                    position: self.position,
                    alpha: self.alpha,
                    solid_color: None,
                    outline: false,
                    rgba_as_mask: false,
                    transform,
                    pip: DrawPipelineId::Text,
                },
                draw,
            );
        });
    }
}

fn add_text_to_batch(
    element: &Text2D,
    quads: &[QuadData],
    transform: corelib::math::Mat3,
    pass: TextPass,
    draw: &mut Draw2D,
) {
    let is_shadow = matches!(pass, TextPass::ShadowOutline | TextPass::ShadowFill);
    let outline = matches!(pass, TextPass::ShadowOutline | TextPass::Outline);
    let offset = if is_shadow {
        element.shadow_offset.unwrap_or(Vec2::ZERO)
    } else {
        Vec2::ZERO
    };
    let solid_color = match pass {
        TextPass::ShadowOutline | TextPass::ShadowFill => Some(element.shadow_color),
        TextPass::Outline => Some(element.outline_color),
        TextPass::Fill => None,
    };

    add_quads_to_batch(
        quads,
        TextBatchPass {
            position: element.position + offset,
            alpha: element.alpha,
            solid_color,
            outline,
            rgba_as_mask: false,
            transform,
            pip: element.pip,
        },
        draw,
    );
}

fn add_quads_to_batch(quads: &[QuadData], pass: TextBatchPass, draw: &mut Draw2D) {
    TEMP_VERTICES.with_borrow_mut(|vertices| {
        TEMP_INDICES.with_borrow_mut(|indices| {
            vertices.clear();
            indices.clear();

            for quad in quads {
                let (xy, quad_size, uvs1, uvs2, source) = if pass.outline {
                    let Some(outline) = &quad.outline else {
                        continue;
                    };
                    (
                        outline.xy + pass.position,
                        outline.size,
                        outline.uvs1,
                        outline.uvs2,
                        outline.source,
                    )
                } else {
                    (
                        quad.xy + pass.position,
                        quad.size,
                        quad.uvs1,
                        quad.uvs2,
                        quad.source,
                    )
                };
                let source = if pass.rgba_as_mask {
                    source.as_mask()
                } else {
                    source
                };
                let color = pass.solid_color.unwrap_or(quad.color);
                let color = color.with_alpha(color.a * pass.alpha);
                push_quad(
                    vertices,
                    indices,
                    xy,
                    quad_size,
                    uvs1,
                    uvs2,
                    source,
                    color,
                    quad.pixelated,
                );
            }
            submit_text_batch(draw, vertices, indices, pass.transform, pass.pip);
        });
    });
}

fn push_quad(
    vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
    xy: Vec2,
    size: Vec2,
    uvs1: Vec2,
    uvs2: Vec2,
    source: crate::text::TextSource,
    color: Color,
    pixelated: bool,
) {
    let xy = if pixelated { xy.round() } else { xy };
    let size = if pixelated { size.round() } else { size };
    let Vec2 { x: x1, y: y1 } = xy;
    let Vec2 { x: x2, y: y2 } = xy + size;
    let Vec2 { x: u1, y: v1 } = uvs1;
    let Vec2 { x: u2, y: v2 } = uvs2;
    let source = source.selector();
    let index = (vertices.len() / 9) as u32;

    #[rustfmt::skip]
    let quad = [
        x1, y1, u1, v1, source, color.r, color.g, color.b, color.a,
        x2, y1, u2, v1, source, color.r, color.g, color.b, color.a,
        x1, y2, u1, v2, source, color.r, color.g, color.b, color.a,
        x2, y2, u2, v2, source, color.r, color.g, color.b, color.a,
    ];
    #[rustfmt::skip]
    let quad_indices = [
        index, index + 1, index + 2,
        index + 2, index + 1, index + 3,
    ];
    vertices.extend_from_slice(&quad);
    indices.extend_from_slice(&quad_indices);
}

fn submit_text_batch(
    draw: &mut Draw2D,
    vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
    transform: corelib::math::Mat3,
    pipeline: DrawPipelineId,
) {
    if vertices.is_empty() {
        return;
    }
    draw.add_to_batch(DrawingInfo {
        pipeline,
        vertices,
        indices,
        transform,
        sprite: None,
    });
}

fn finalized_text_transform(size: Vec2, transform: Option<Transform2D>) -> corelib::math::Mat3 {
    let mut transform = transform.unwrap_or_default();
    transform.set_size(size);
    transform.updated_mat3()
}

fn set_text_bounds(draw: &mut Draw2D, position: Vec2, size: Vec2, matrix: corelib::math::Mat3) {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for corner in [
        position,
        position + vec2(size.x, 0.0),
        position + vec2(0.0, size.y),
        position + size,
    ] {
        let point = matrix * vec3(corner.x, corner.y, 1.0);
        min = min.min(vec2(point.x, point.y));
        max = max.max(vec2(point.x, point.y));
    }
    draw.last_text_bounds = Rect::from_min_max(min, max);
}
