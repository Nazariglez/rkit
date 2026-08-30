use crate::text::{
    Font, HAlign, QuadData, RichTextLayout, TextInfo, TextPrepareScratch, TextSystem,
    get_mut_text_system,
};
use crate::{Draw2D, DrawPipelineId, DrawingInfo, Element2D, PipelineContext, Transform2D};
use corelib::gfx::{
    self, BindGroupLayout, BindingType, BlendMode, Buffer, Color, VertexFormat, VertexLayout,
};
use corelib::math::{IntoVec2, Rect, Vec2, bvec2, vec2, vec3};
use macros::Drawable2D;
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

#[derive(Default)]
struct TextDrawScratch {
    prepare: TextPrepareScratch,
    quads: Vec<QuadData>,
    vertices: Vec<f32>,
    indices: Vec<u32>,
}

thread_local! {
    static TEXT_SCRATCH: RefCell<Vec<TextDrawScratch>> = const { RefCell::new(Vec::new()) };
}

struct ScratchLease(Option<TextDrawScratch>);

impl ScratchLease {
    fn take() -> Self {
        Self(Some(
            TEXT_SCRATCH.with(|pool| pool.borrow_mut().pop().unwrap_or_default()),
        ))
    }
}

impl Deref for ScratchLease {
    type Target = TextDrawScratch;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

impl DerefMut for ScratchLease {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap()
    }
}

impl Drop for ScratchLease {
    fn drop(&mut self) {
        let mut scratch = self.0.take().unwrap();
        scratch.quads.clear();
        scratch.vertices.clear();
        scratch.indices.clear();
        TEXT_SCRATCH.with(|pool| pool.borrow_mut().push(scratch));
    }
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
var t_mask_linear: texture_2d<f32>;
@group(1) @binding(3)
var t_mask_nearest: texture_2d<f32>;
@group(1) @binding(4)
var t_rgba_linear: texture_2d<f32>;
@group(1) @binding(5)
var t_rgba_nearest: texture_2d<f32>;

// srg to linear
{{SRGB_TO_LINEAR}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let in_color = srgb_to_linear(in.color);

    {{SELECT_TEXTURE_AND_SAMPLER}}
}
"#;

const SELECT_TEXTURE_SAMPLER: &str = r#"
    if (in.tex == 6.0) {
        return in_color;
    }
    if (in.tex == 0.0) {
        let mask = textureSampleLevel(t_mask_linear, s_linear, in.uvs, 0.0);
        return vec4(in_color.rgb, mask.r * in_color.a);
    }
    if (in.tex == 1.0) {
        let mask = textureSampleLevel(t_mask_nearest, s_nearest, in.uvs, 0.0);
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

pub fn create_text_2d_pipeline_ctx(ubo_transform: &Buffer) -> Result<PipelineContext, String> {
    let shader = SHADER
        .replace(
            "{{SRGB_TO_LINEAR}}",
            include_str!("../resources/to_linear.wgsl"),
        )
        .replace("{{SELECT_TEXTURE_AND_SAMPLER}}", SELECT_TEXTURE_SAMPLER);

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
                .with_entry(BindingType::texture(4).with_fragment_visibility(true))
                .with_entry(BindingType::texture(5).with_fragment_visibility(true)),
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
        let mut scratch = ScratchLease::take();
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
            None => match draw
                .target_resolution(transform * corelib::math::Mat3::from_translation(self.position))
            {
                Ok(resolution) => TextSystem::automatic_resolution(resolution),
                Err(error) => {
                    system.recycle_layout(layout);
                    drop(system);
                    draw.record_error(error);
                    return;
                }
            },
        };
        let scratch = &mut *scratch;
        let prepared = layout
            .run_effects(0.0, 0, None, &mut scratch.prepare)
            .and_then(|()| {
                system.prepare_layout(
                    &layout,
                    resolution,
                    &mut scratch.prepare,
                    &mut scratch.quads,
                )
            });
        system.recycle_layout(layout);
        drop(system);
        if let Err(error) = prepared {
            draw.record_error(error);
            return;
        }

        set_text_bounds(draw, self.position, size, transform);
        if scratch.quads.is_empty() {
            return;
        }
        let outlined = self.outline_width > 0;
        if self.shadow_offset.is_some() {
            if outlined {
                add_text_to_batch(self, transform, TextPass::ShadowOutline, scratch, draw);
            }
            add_text_to_batch(self, transform, TextPass::ShadowFill, scratch, draw);
        }
        if outlined {
            add_text_to_batch(self, transform, TextPass::Outline, scratch, draw);
        }
        add_text_to_batch(self, transform, TextPass::Fill, scratch, draw);
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
    reveal: Option<usize>,
    effect_time: f32,
    effect_seed: u64,

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
            reveal: None,
            effect_time: 0.0,
            effect_seed: 0,
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

    pub fn reveal(&mut self, units: usize) -> &mut Self {
        self.reveal = Some(units);
        self
    }

    pub fn effect_time(&mut self, time: f32) -> &mut Self {
        self.effect_time = time;
        self
    }

    pub fn effect_seed(&mut self, seed: u64) -> &mut Self {
        self.effect_seed = seed;
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

        let mut scratch = ScratchLease::take();
        let scratch = &mut *scratch;
        let prepared = self.layout.layout.run_effects(
            self.effect_time,
            self.effect_seed,
            self.reveal,
            &mut scratch.prepare,
        );
        if let Err(error) = prepared {
            draw.record_error(error);
            return;
        }
        let mut system = get_mut_text_system();
        let prepared = system.prepare_layout(
            &self.layout.layout,
            resolution,
            &mut scratch.prepare,
            &mut scratch.quads,
        );
        drop(system);
        if let Err(error) = prepared {
            draw.record_error(error);
            return;
        }

        let bounds = match effect_text_bounds(scratch, self.position, size, transform) {
            Ok(bounds) => bounds,
            Err(error) => {
                draw.record_error(error);
                return;
            }
        };
        draw.last_text_bounds = bounds;
        if let Some(offset) = self.shadow_offset {
            add_quads_to_batch(
                TextBatchPass {
                    position: self.position + offset,
                    alpha: self.alpha,
                    solid_color: Some(self.shadow_color),
                    outline: false,
                    rgba_as_mask: true,
                    transform,
                    pip: DrawPipelineId::Text,
                },
                scratch,
                draw,
            );
        }
        add_quads_to_batch(
            TextBatchPass {
                position: self.position,
                alpha: self.alpha,
                solid_color: None,
                outline: false,
                rgba_as_mask: false,
                transform,
                pip: DrawPipelineId::Text,
            },
            scratch,
            draw,
        );
    }
}

fn add_text_to_batch(
    element: &Text2D,
    transform: corelib::math::Mat3,
    pass: TextPass,
    scratch: &mut TextDrawScratch,
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
        TextBatchPass {
            position: element.position + offset,
            alpha: element.alpha,
            solid_color,
            outline,
            rgba_as_mask: false,
            transform,
            pip: element.pip,
        },
        scratch,
        draw,
    );
}

fn add_quads_to_batch(pass: TextBatchPass, scratch: &mut TextDrawScratch, draw: &mut Draw2D) {
    scratch.vertices.clear();
    scratch.indices.clear();

    for quad in &scratch.quads {
        let Some(state) = scratch.prepare.state(quad.atom) else {
            continue;
        };
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
        if pass.rgba_as_mask && source == crate::text::TextSource::Solid {
            continue;
        }
        let source = if pass.rgba_as_mask {
            source.as_mask()
        } else {
            source
        };
        let color = match pass.solid_color {
            Some(color) => color.with_alpha(color.a * state.alpha * pass.alpha),
            None => quad.color.with_alpha(quad.color.a * pass.alpha),
        };
        push_quad(
            &mut scratch.vertices,
            &mut scratch.indices,
            xy,
            quad_size,
            uvs1,
            uvs2,
            source,
            color,
            quad.pixelated,
            state,
            pass.position,
        );
    }
    submit_text_batch(
        draw,
        &mut scratch.vertices,
        &mut scratch.indices,
        pass.transform,
        pass.pip,
    );
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
    state: crate::text::PreparedAtom,
    effect_origin: Vec2,
) {
    let xy = if pixelated { xy.round() } else { xy };
    let size = if pixelated { size.round() } else { size };
    let top_left = effect_point(xy, state, effect_origin);
    let top_right = effect_point(xy + vec2(size.x, 0.0), state, effect_origin);
    let bottom_left = effect_point(xy + vec2(0.0, size.y), state, effect_origin);
    let bottom_right = effect_point(xy + size, state, effect_origin);
    let Vec2 { x: u1, y: v1 } = uvs1;
    let Vec2 { x: u2, y: v2 } = uvs2;
    let source = source.selector();
    let index = (vertices.len() / 9) as u32;

    #[rustfmt::skip]
    let quad = [
        top_left.x, top_left.y, u1, v1, source, color.r, color.g, color.b, color.a,
        top_right.x, top_right.y, u2, v1, source, color.r, color.g, color.b, color.a,
        bottom_left.x, bottom_left.y, u1, v2, source, color.r, color.g, color.b, color.a,
        bottom_right.x, bottom_right.y, u2, v2, source, color.r, color.g, color.b, color.a,
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

fn effect_point(point: Vec2, state: crate::text::PreparedAtom, origin: Vec2) -> Vec2 {
    let center = state.center + origin;
    let point = (point - center) * state.scale;
    let (sin, cos) = state.rotation.sin_cos();
    vec2(point.x * cos - point.y * sin, point.x * sin + point.y * cos) + center + state.translation
}

fn effect_text_bounds(
    scratch: &TextDrawScratch,
    position: Vec2,
    size: Vec2,
    matrix: corelib::math::Mat3,
) -> Result<Rect, String> {
    if scratch.quads.len() > u32::MAX as usize / 4 {
        return Err("Text geometry exceeds index limits".into());
    }
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    let mut add = |point: Vec2| -> Result<(), String> {
        let point = matrix * vec3(point.x, point.y, 1.0);
        if ![point.x, point.y].into_iter().all(f32::is_finite) {
            return Err("Text effect produced non-finite geometry".into());
        }
        let point = vec2(point.x, point.y);
        min = min.min(point);
        max = max.max(point);
        Ok(())
    };
    for corner in [
        position,
        position + vec2(size.x, 0.0),
        position + vec2(0.0, size.y),
        position + size,
    ] {
        add(corner)?;
    }
    for quad in &scratch.prepare.bounds_quads {
        let Some(state) = scratch.prepare.state(quad.atom) else {
            return Err("Text prepared atom is missing".into());
        };
        for corner in [
            quad.xy + position,
            quad.xy + position + vec2(quad.size.x, 0.0),
            quad.xy + position + vec2(0.0, quad.size.y),
            quad.xy + position + quad.size,
        ] {
            add(effect_point(corner, state, position))?;
        }
    }
    Ok(Rect::from_min_max(min, max))
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
