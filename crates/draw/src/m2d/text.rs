use crate::text::{Font, HAlign, QuadData, RichTextLayout, TextInfo, get_mut_text_system};
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
    return textureSampleLevel(t_rgba_nearest, s_nearest, in.uvs, 0.0) * in_color;
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
    return textureSampleLevel(t_rgba_nearest, s_nearest, in.uvs, 0.0) * in_color;
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
    res: f32,
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
            res: 1.0,
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

    pub fn resolution(&mut self, res: f32) -> &mut Self {
        self.res = res;
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

impl Element2D for Text2D<'_> {
    fn process(&self, draw: &mut Draw2D) {
        let info = TextInfo {
            font: self.font,
            text: self.text,
            wrap_width: self.max_width,
            font_size: self.size,
            line_height: self.line_height,
            resolution: self.res,
            h_align: self.h_align,
            color_tags: self.color_tags,
            default_color: self.color,
            outline_width: self.outline_width,
            strict_metrics: false,
        };
        TEMP_QUADS.with_borrow_mut(|quads| {
            let size = {
                let mut system = get_mut_text_system();
                let mut layout = system.take_layout();
                system.layout_text(&info, &mut layout).unwrap();
                system.ensure_layout(&layout).unwrap();
                system.resolve_layout(&layout, quads);
                let size = layout.size;
                system.recycle_layout(layout);
                size
            };
            set_text_bounds(
                draw,
                self.position,
                size,
                self.transform.unwrap_or_default(),
            );
            if quads.is_empty() {
                return;
            }
            let outlined = self.outline_width > 0;
            if self.shadow_offset.is_some() {
                if outlined {
                    add_text_to_batch(self, quads, size, TextPass::ShadowOutline, draw);
                }
                add_text_to_batch(self, quads, size, TextPass::ShadowFill, draw);
            }
            if outlined {
                add_text_to_batch(self, quads, size, TextPass::Outline, draw);
            }
            add_text_to_batch(self, quads, size, TextPass::Fill, draw);
        });
    }
}

#[derive(Drawable2D)]
pub struct RichText2D<'a> {
    layout: &'a RichTextLayout,
    position: Vec2,
    alpha: f32,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl<'a> RichText2D<'a> {
    pub(crate) fn new(layout: &'a RichTextLayout) -> Self {
        Self {
            layout,
            position: Vec2::ZERO,
            alpha: 1.0,
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
}

impl Element2D for RichText2D<'_> {
    fn process(&self, draw: &mut Draw2D) {
        let size = self.layout.size();
        let transform = self.transform.unwrap_or_default();
        set_text_bounds(draw, self.position, size, transform);

        TEMP_QUADS.with_borrow_mut(|quads| {
            let mut system = get_mut_text_system();
            system.ensure_layout(&self.layout.layout).unwrap();
            system.resolve_layout(&self.layout.layout, quads);
            drop(system);
            add_rich_text_to_batch(self, quads, size, draw);
        });
    }
}

fn add_text_to_batch(
    element: &Text2D,
    quads: &[QuadData],
    size: Vec2,
    pass: TextPass,
    draw: &mut Draw2D,
) {
    let is_shadow = matches!(pass, TextPass::ShadowOutline | TextPass::ShadowFill);
    let is_outline = matches!(pass, TextPass::ShadowOutline | TextPass::Outline);
    let shadow_color = element
        .shadow_color
        .with_alpha(element.shadow_color.a * element.alpha);
    let outline_color = element
        .outline_color
        .with_alpha(element.outline_color.a * element.alpha);
    let offset = if is_shadow {
        element.shadow_offset.unwrap_or(Vec2::ZERO)
    } else {
        Vec2::ZERO
    };
    let position = element.position + offset;

    TEMP_VERTICES.with_borrow_mut(|vertices| {
        TEMP_INDICES.with_borrow_mut(|indices| {
            vertices.clear();
            indices.clear();

            for quad in quads {
                let (xy, quad_size, uvs1, uvs2, source) = if is_outline {
                    let Some(outline) = &quad.outline else {
                        continue;
                    };
                    (
                        outline.xy + position,
                        outline.size,
                        outline.uvs1,
                        outline.uvs2,
                        outline.source,
                    )
                } else {
                    (
                        quad.xy + position,
                        quad.size,
                        quad.uvs1,
                        quad.uvs2,
                        quad.source,
                    )
                };
                let color = match pass {
                    TextPass::ShadowOutline | TextPass::ShadowFill => shadow_color,
                    TextPass::Outline => outline_color,
                    TextPass::Fill => quad.color.with_alpha(quad.color.a * element.alpha),
                };
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
            submit_text_batch(
                draw,
                vertices,
                indices,
                size,
                element.transform.unwrap_or_default(),
                element.pip,
            );
        });
    });
}

fn add_rich_text_to_batch(element: &RichText2D, quads: &[QuadData], size: Vec2, draw: &mut Draw2D) {
    TEMP_VERTICES.with_borrow_mut(|vertices| {
        TEMP_INDICES.with_borrow_mut(|indices| {
            vertices.clear();
            indices.clear();
            for quad in quads {
                let color = quad.color.with_alpha(quad.color.a * element.alpha);
                push_quad(
                    vertices,
                    indices,
                    quad.xy + element.position,
                    quad.size,
                    quad.uvs1,
                    quad.uvs2,
                    quad.source,
                    color,
                    quad.pixelated,
                );
            }
            submit_text_batch(
                draw,
                vertices,
                indices,
                size,
                element.transform.unwrap_or_default(),
                DrawPipelineId::Text,
            );
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
    size: Vec2,
    mut transform: Transform2D,
    pipeline: DrawPipelineId,
) {
    if vertices.is_empty() {
        return;
    }
    transform.set_size(size);
    draw.add_to_batch(DrawingInfo {
        pipeline,
        vertices,
        indices,
        transform: transform.updated_mat3(),
        sprite: None,
    });
}

fn set_text_bounds(draw: &mut Draw2D, position: Vec2, size: Vec2, mut transform: Transform2D) {
    transform.set_size(size);
    let matrix = transform.updated_mat3();
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
