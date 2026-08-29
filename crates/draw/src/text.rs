use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use corelib::gfx::{
    self, BindGroup, Color, RenderPipeline, RenderTexture, Sampler, Texture, TextureFilter,
    TextureFormat, TextureId,
};
use corelib::math::{UVec2, Vec2, uvec2, vec2};
use cosmic_text::fontdb::Source;
use cosmic_text::{
    Attrs, Buffer, CacheKey, Family, FontSystem, Hinting, Metrics, Shaping, Stretch, Style,
    SwashCache, SwashContent, UnderlineStyle, Weight, Wrap,
};
use etagere::{BucketedAtlasAllocator, size2};
use markup::MarkupMode;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use std::sync::Arc;

mod document;
mod effect;
mod icon_baker;
mod layout;
mod markup;
mod render;
mod rich;
mod shaping;
mod style;

pub use document::{
    RichTextDocument, TextDiagnostic, TextDiagnosticCode, TextDiagnosticSeverity, TextMarkupPolicy,
    TextSourceId,
};
pub(crate) use effect::PreparedAtom;
pub use effect::{TextEffect, TextEffectItem, TextEffectItems, TextEffectRun, TextEffects};
use icon_baker::{IconBake, IconBaker};
use layout::{AtomKind, LayoutAtoms, LineGeometry, LogicalBounds, NewAtom};
use render::{PlacedGlyph, PlacedIcon, PlacedSolid, RenderItem, TextRenderPlan};
use rich::{PixelRect, RegisteredIcon};
pub use rich::{
    RichDocumentBuilder, RichTextAtomKind, RichTextBuilder, RichTextHit, RichTextIcon,
    RichTextLayout, RichTextLine, TextAffinity, TextIconAlign, TextIcons, rich_document, rich_text,
};
pub use style::{TextStyle, TextStyles};
use utils::helpers::closest_multiple_of;

pub(crate) static TEXT_SYSTEM: Lazy<AtomicRefCell<TextSystem>> =
    Lazy::new(|| AtomicRefCell::new(TextSystem::new().unwrap()));

#[cfg(target_arch = "wasm32")]
unsafe impl Sync for TextSystem {}
#[cfg(target_arch = "wasm32")]
unsafe impl Send for TextSystem {}

pub fn get_text_system() -> AtomicRef<'static, TextSystem> {
    TEXT_SYSTEM.borrow()
}

pub fn get_mut_text_system() -> AtomicRefMut<'static, TextSystem> {
    TEXT_SYSTEM.borrow_mut()
}

const DEFAULT_TEXTURE_SIZE: u32 = 256;
const ATLAS_PIXEL_OFFSET: u32 = 1;
const MAX_OUTLINE_BUFFER_BYTES: usize = 64 * 1024 * 1024;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct FontId(pub(crate) u64);

#[derive(Clone, Debug)]
pub struct Font {
    id: FontId,
    _raw: cosmic_text::fontdb::ID,
    nearest: bool,
    // designed ppem
    res_ppem: f32,
    px_per_em: f32,
    line_height_pem: f32,
    family: Arc<String>,
    weight: Weight,
    style: Style,
    stretch: Stretch,
    // TODO DropObserver, seems that cosmic-text doesn't have a way to remove fonts right now
}

impl Font {
    pub fn id(&self) -> FontId {
        self.id
    }

    pub fn is_pixelated(&self) -> bool {
        self.nearest
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OutlineQuad {
    pub(crate) xy: Vec2,
    pub(crate) size: Vec2,
    pub(crate) uvs1: Vec2,
    pub(crate) uvs2: Vec2,
    pub(crate) source: TextSource,
}

#[derive(Clone, Debug)]
pub(crate) struct QuadData {
    pub(crate) atom: usize,
    pub(crate) xy: Vec2,
    pub(crate) size: Vec2,
    pub(crate) uvs1: Vec2,
    pub(crate) uvs2: Vec2,
    pub(crate) source: TextSource,
    pub(crate) color: Color,
    pub(crate) pixelated: bool,
    pub(crate) outline: Option<OutlineQuad>,
}

pub(crate) struct TextLayout {
    pub(crate) size: Vec2,
    pub(crate) lines: Vec<rich::RichTextLine>,
    semantic_text: String,
    plan: TextRenderPlan,
    atoms: LayoutAtoms,
    effect_callbacks: Vec<std::sync::Arc<effect::EffectCallback>>,
    effects: Vec<effect::EffectOccurrence>,
    outline_width: u16,
}

impl Default for TextLayout {
    fn default() -> Self {
        Self {
            size: Vec2::ZERO,
            lines: Vec::new(),
            semantic_text: String::new(),
            plan: TextRenderPlan::default(),
            atoms: LayoutAtoms::default(),
            effect_callbacks: Vec::new(),
            effects: Vec::new(),
            outline_width: 0,
        }
    }
}

impl TextLayout {
    fn clear(&mut self) {
        self.size = Vec2::ZERO;
        self.lines.clear();
        self.semantic_text.clear();
        self.plan.clear();
        self.atoms.clear();
        self.effect_callbacks.clear();
        self.effects.clear();
        self.outline_width = 0;
    }

    pub(crate) fn run_effects(
        &self,
        time: f32,
        seed: u64,
        reveal: Option<usize>,
        scratch: &mut TextPrepareScratch,
    ) -> Result<(), String> {
        if !time.is_finite() {
            return Err("Text effect time must be finite".into());
        }
        scratch.states.clear();
        scratch.states.reserve(self.atoms.atom_count());
        for index in 0..self.atoms.atom_count() {
            let atom = self
                .atoms
                .logical(index)
                .ok_or_else(|| "Text atom order is invalid".to_string())?;
            let rect = atom.rect();
            let center = rect.origin + rect.size * 0.5;
            let color = self
                .atoms
                .logical_color(index)
                .ok_or_else(|| "Text atom order is invalid".to_string())?;
            let hidden = reveal.is_some_and(|limit| index >= limit);
            scratch
                .states
                .push(effect::PreparedAtom::new(color, hidden, center));
        }
        for (occurrence_index, occurrence) in self.effects.iter().enumerate() {
            let states = scratch
                .states
                .get_mut(occurrence.atoms.clone())
                .ok_or_else(|| "Text effect atom range is invalid".to_string())?;
            let callback = self
                .effect_callbacks
                .get(occurrence.callback)
                .ok_or_else(|| "Text effect callback is missing".to_string())?;
            callback(effect::TextEffectRun {
                time,
                seed,
                occurrence_index,
                text: &self.semantic_text,
                atoms: &self.atoms,
                start: occurrence.atoms.start,
                states,
            });
        }
        if scratch.states.iter().copied().any(|state| !state.valid()) {
            return Err("Text effect produced a non-finite value".into());
        }
        Ok(())
    }
}

enum RasterItem {
    Glyph {
        atom: usize,
        key: CacheKey,
        pos: Vec2,
        scale: f32,
        outline_radius: u16,
        color: Color,
        pixelated: bool,
    },
    Icon {
        index: usize,
        color: Color,
        atom: usize,
    },
    Solid {
        atom: usize,
        pos: Vec2,
        size: Vec2,
        color: Color,
        pixelated: bool,
    },
}

impl RasterItem {
    fn pixelated(&self) -> bool {
        match self {
            Self::Glyph { pixelated, .. } => *pixelated,
            Self::Icon { .. } => false,
            Self::Solid { pixelated, .. } => *pixelated,
        }
    }
}

#[derive(Default)]
pub(crate) struct TextPrepareScratch {
    items: Vec<RasterItem>,
    states: Vec<effect::PreparedAtom>,
}

impl TextPrepareScratch {
    pub(crate) fn state(&self, index: usize) -> Option<PreparedAtom> {
        self.states.get(index).copied()
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Default)]
pub enum HAlign {
    #[default]
    Left,
    Center,
    Right,
}

pub(crate) struct TextInfo<'a> {
    pub(crate) font: Option<&'a Font>,
    pub(crate) text: &'a str,
    pub(crate) wrap_width: Option<f32>,
    pub(crate) font_size: f32,
    pub(crate) line_height: Option<f32>,
    pub(crate) h_align: HAlign,
    pub(crate) color_tags: bool,
    pub(crate) default_color: Color,
    pub(crate) outline_width: u16,
    pub(crate) strict_metrics: bool,
}

pub fn text_metrics(text: &str) -> TextMetricsBuilder<'_> {
    TextMetricsBuilder {
        info: TextInfo {
            font: None,
            text,
            wrap_width: None,
            font_size: 14.0,
            line_height: None,
            h_align: Default::default(),
            color_tags: false,
            default_color: Color::WHITE,
            outline_width: 0,
            strict_metrics: false,
        },
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TextMetrics {
    pub size: Vec2,
    pub lines: usize,
}

pub struct TextMetricsBuilder<'a> {
    info: TextInfo<'a>,
}

impl<'a> TextMetricsBuilder<'a> {
    pub fn font(mut self, font: &'a Font) -> Self {
        self.info.font = Some(font);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.info.font_size = size;
        self
    }

    pub fn max_width(mut self, width: f32) -> Self {
        self.info.wrap_width = Some(width);
        self
    }

    pub fn line_height(mut self, height: f32) -> Self {
        self.info.line_height = Some(height);
        self
    }

    pub fn color_tags(mut self) -> Self {
        self.info.color_tags = true;
        self
    }

    pub fn default_color(mut self, color: Color) -> Self {
        self.info.default_color = color;
        self
    }

    pub fn outline(mut self, width: u16) -> Self {
        self.info.outline_width = width;
        self
    }

    pub fn measure(self) -> TextMetrics {
        let mut system = get_mut_text_system();
        let mut layout = system.take_layout();
        system.layout_text(&self.info, &mut layout).unwrap();
        let metrics = TextMetrics {
            size: layout.size,
            lines: layout.lines.len(),
        };
        system.recycle_layout(layout);
        metrics
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
struct GlyphCacheKey {
    key: CacheKey,
    outline: u16,
    pixelated: bool,
}

pub struct TextSystem {
    atlases: [AtlasData; TextAtlas::COUNT],
    linear_sampler: Sampler,
    nearest_sampler: Sampler,
    cache: FxHashMap<GlyphCacheKey, GlyphInfo>,
    icon_cache: FxHashMap<IconCacheKey, IconInfo>,
    pending_icons: FxHashMap<IconCacheKey, u32>,
    icon_baker: IconBaker,
    font_system: FontSystem,
    swash: SwashCache,
    buffer: Buffer,
    font_ids: u64,
    default_font: Option<Font>,

    bind_group: Option<BindGroup>,

    // reusable buffer used to avoid per glyph allocations
    temp_outline_buff: Vec<u8>,
    temp_layout: TextLayout,
}

fn record_effect_atoms(
    occurrences: &mut [Vec<layout::AtomId>],
    effects: &[u32],
    atom: layout::AtomId,
) -> Result<(), String> {
    for occurrence in effects {
        let atoms = occurrences
            .get_mut(*occurrence as usize)
            .ok_or_else(|| "Text effect occurrence is out of bounds".to_string())?;
        if atoms.last().copied() != Some(atom) {
            atoms.push(atom);
        }
    }
    Ok(())
}

fn push_decorations(
    plan: &mut TextRenderPlan,
    atom: layout::AtomId,
    x: f32,
    width: f32,
    baseline: f32,
    font_size: f32,
    underline: bool,
    strikethrough: bool,
    pixelated: bool,
    decoration: Option<&cosmic_text::DecorationSpan>,
) {
    if width <= 0.0 {
        return;
    }
    let metrics = decoration.map(|decoration| &decoration.data);
    if underline {
        let offset = metrics.map_or(-0.125, |data| data.underline_metrics.offset);
        let thickness = metrics.map_or(1.0 / 14.0, |data| data.underline_metrics.thickness);
        plan.push(RenderItem::Solid(PlacedSolid {
            atom,
            pos: vec2(x, baseline - offset * font_size),
            size: vec2(width, thickness * font_size),
            pixelated,
        }));
    }
    if strikethrough {
        let offset = metrics.map_or(0.3, |data| data.strikethrough_metrics.offset);
        let thickness = metrics.map_or(1.0 / 14.0, |data| data.strikethrough_metrics.thickness);
        plan.push(RenderItem::Solid(PlacedSolid {
            atom,
            pos: vec2(x, baseline - offset * font_size),
            size: vec2(width, thickness * font_size),
            pixelated,
        }));
    }
}

impl TextSystem {
    pub fn new() -> Result<Self, String> {
        // common
        let max_texture_size = gfx::limits().max_texture_size_2d;
        let linear_sampler = gfx::create_sampler()
            .with_label("TextSystem Linear Sampler")
            .with_min_filter(TextureFilter::Linear)
            .with_mag_filter(TextureFilter::Linear)
            .build()?;

        let nearest_sampler = gfx::create_sampler()
            .with_label("TextSystem Nearest Sampler")
            .with_min_filter(TextureFilter::Nearest)
            .with_mag_filter(TextureFilter::Nearest)
            .build()?;

        let mask_linear = AtlasData::new(
            "TextSystem Texture Mask Linear",
            TextureFormat::R8UNorm,
            max_texture_size,
        )?;
        let mask_nearest = AtlasData::new(
            "TextSystem Texture Mask Nearest",
            TextureFormat::R8UNorm,
            max_texture_size,
        )?;
        let rgba_linear = AtlasData::new(
            "TextSystem Texture RGBA Linear",
            TextureFormat::Rgba8UNormSrgb,
            max_texture_size,
        )?;
        let rgba_nearest = AtlasData::new(
            "TextSystem Texture RGBA Nearest",
            TextureFormat::Rgba8UNormSrgb,
            max_texture_size,
        )?;

        let cache = FxHashMap::default();

        let mut font_system = FontSystem::new();
        let swash = SwashCache::new();

        let buffer = Buffer::new(&mut font_system, Metrics::new(1.0, 1.0));

        #[allow(unused_mut)]
        let mut sys = Self {
            atlases: [mask_linear, mask_nearest, rgba_linear, rgba_nearest],
            linear_sampler,
            nearest_sampler,
            cache,
            icon_cache: FxHashMap::default(),
            pending_icons: FxHashMap::default(),
            icon_baker: IconBaker::new()?,
            font_system,
            swash,
            buffer,
            font_ids: 0,
            default_font: None,

            bind_group: None,

            temp_outline_buff: vec![],
            temp_layout: TextLayout::default(),
        };

        #[cfg(feature = "default-font")]
        {
            let font = sys.create_font(
                include_bytes!("./resources/arcade-legacy/arcade-legacy.ttf"),
                true,
                None,
            )?;
            sys.default_font = Some(font);
        }

        Ok(sys)
    }

    pub fn bind_group(&mut self, pip: &RenderPipeline) -> &BindGroup {
        if self.bind_group.is_none() {
            log::debug!("New text atlas bind_group created");
            let bg = gfx::create_bind_group()
                .with_label("TextSystem BindGroup")
                .with_sampler(0, &self.linear_sampler)
                .with_sampler(1, &self.nearest_sampler)
                .with_texture(2, &self.atlas(TextAtlas::MaskLinear).texture)
                .with_texture(3, &self.atlas(TextAtlas::MaskNearest).texture)
                .with_texture(4, &self.atlas(TextAtlas::RgbaLinear).texture)
                .with_texture(5, &self.atlas(TextAtlas::RgbaNearest).texture)
                .with_layout(pip.bind_group_layout_ref(1).unwrap())
                .build()
                .unwrap();

            self.bind_group = Some(bg);
        }

        self.bind_group.as_ref().unwrap()
    }

    pub fn set_default_font(&mut self, font: &Font) {
        self.default_font = Some(font.clone());
    }

    pub fn create_font(
        &mut self,
        data: &[u8],
        nearest: bool,
        line_height_pem: Option<f32>,
    ) -> Result<Font, String> {
        if let Some(line_height_pem) = line_height_pem {
            validate_positive(line_height_pem, "Text font line-height scale")?;
        }

        let id = self.font_ids;
        self.font_ids += 1;
        let ids = self
            .font_system
            .db_mut()
            .load_font_source(Source::Binary(Arc::new(data.to_vec())));
        let raw_id = *ids
            .first()
            .ok_or_else(|| "Cannot create the font".to_string())?;

        let face = self
            .font_system
            .db()
            .face(raw_id)
            .cloned()
            .ok_or_else(|| "Invalid font type".to_string())?;
        let mut suffix = 0_u64;
        let family = loop {
            let family = format!("__rkit_font_{id}_{suffix}");
            let collision = self
                .font_system
                .db()
                .faces()
                .any(|face| face.families.iter().any(|candidate| candidate.0 == family));
            if !collision {
                break family;
            }
            suffix = suffix
                .checked_add(1)
                .ok_or_else(|| "Cannot allocate an exact font alias".to_string())?;
        };
        let mut alias = face.clone();
        alias.id = cosmic_text::fontdb::ID::dummy();
        alias.families = vec![(
            family.clone(),
            cosmic_text::fontdb::Language::English_UnitedStates,
        )];
        self.font_system.db_mut().remove_face(raw_id);
        let alias_id = self.font_system.db_mut().push_face_info(alias);
        let font = self
            .font_system
            .get_font(alias_id, face.weight)
            .ok_or_else(|| "Invalid font id".to_string())?;
        let font_ref = font.as_swash();
        let metrics = font_ref.metrics(&[]);

        // FIXME: use ttf-parser to get 'resPPEM' directly from the font
        // I am just using 8.0 because it seems to be a common value for pixel fonts
        let res_ppem = if nearest { 8.0 } else { 16.0 };

        // calculate scale values to make the font look
        // right later when processing the glyphs
        let upm = metrics.units_per_em as f32;
        let asc = metrics.ascent;
        let desc = metrics.descent;
        let lead = metrics.leading;
        let metrics_lh_pem = (asc + desc + lead) / upm;
        validate_positive(metrics_lh_pem, "Text font line-height scale")?;
        let line_height_pem = line_height_pem.unwrap_or(metrics_lh_pem);
        let px_per_em = upm / metrics.cap_height;
        validate_positive(px_per_em, "Text font pixel scale")?;

        Ok(Font {
            id: FontId(id),
            _raw: alias_id,
            nearest,
            res_ppem,
            px_per_em,
            line_height_pem,
            family: Arc::new(family),
            weight: face.weight,
            style: face.style,
            stretch: face.stretch,
        })
    }

    pub(crate) fn take_layout(&mut self) -> TextLayout {
        std::mem::take(&mut self.temp_layout)
    }

    pub(crate) fn recycle_layout(&mut self, mut layout: TextLayout) {
        layout.clear();
        self.temp_layout = layout;
    }

    pub(crate) fn layout_text(
        &mut self,
        text: &TextInfo,
        layout: &mut TextLayout,
    ) -> Result<(), String> {
        let mut document = if text.color_tags {
            markup::parse(
                text.text,
                text.default_color,
                MarkupMode::Colors,
                text.wrap_width.is_some(),
            )
        } else {
            markup::plain(text.text, text.default_color, text.wrap_width.is_some())
        };
        let mut diagnostics = std::mem::take(&mut document.diagnostics);
        let shaping = shaping::prepare(document, text.wrap_width.is_some(), &mut diagnostics)?;
        self.layout_document(text, shaping, layout, &mut diagnostics)
    }

    pub(crate) fn layout_document(
        &mut self,
        text: &TextInfo,
        markup: shaping::ShapingInput,
        layout: &mut TextLayout,
        diagnostics: &mut document::DiagnosticSink,
    ) -> Result<(), String> {
        layout.clear();
        layout.semantic_text.push_str(&markup.semantic_text);
        let font = text.font.or(self.default_font.as_ref());
        let (pixelated, ppem, res_ppem, line_height_pem) = font
            .map(|font| {
                (
                    font.is_pixelated(),
                    font.px_per_em,
                    font.res_ppem,
                    font.line_height_pem,
                )
            })
            .unwrap_or((false, 1.0, 1.0, 1.0));
        let attrs = font_attrs(font);
        let (font_size, _strike_scale, base_line_height) =
            validate_layout_metrics(text, pixelated, ppem, res_ppem, line_height_pem)?;
        self.buffer
            .set_metrics(Metrics::new(font_size, base_line_height));
        self.buffer.set_size(text.wrap_width, None);

        let mut span_profiles = Vec::with_capacity(markup.spans.len());
        let mut spans = Vec::with_capacity(markup.spans.len());
        for (index, span) in markup.spans.iter().enumerate() {
            let span_font = span.font.as_ref().or(font);
            let logical_size = span.size.unwrap_or(text.font_size);
            let logical_line_height = span.line_height.or(text.line_height);
            let (span_size, span_scale, span_line_height) =
                validate_style_metrics(span_font, logical_size, logical_line_height)?;
            let span_pixelated = span_font.is_some_and(Font::is_pixelated);
            let underline = if span.underline {
                UnderlineStyle::Single
            } else {
                UnderlineStyle::None
            };
            let mut span_attrs = font_attrs(span_font)
                .metrics(Metrics::new(span_size, span_line_height))
                .underline(underline);
            if span.strikethrough {
                span_attrs = span_attrs.strikethrough();
            }
            let span_attrs = span_attrs.metadata(index + 1);
            span_profiles.push((span_pixelated, span_scale, span_line_height));
            spans.push((&markup.text[span.range.clone()], span_attrs));
        }
        self.buffer
            .set_rich_text(spans, &attrs, Shaping::Advanced, None);
        self.buffer.shape_until_scroll(&mut self.font_system, false);

        let objects: Vec<_> = markup
            .objects
            .into_iter()
            .map(|object| {
                let style = markup
                    .styles
                    .get(object.style.0)
                    .ok_or_else(|| "Text icon has an invalid resolved style".to_string())?;
                let height = object
                    .options
                    .height
                    .or(style.size)
                    .unwrap_or(text.font_size);
                let object_font = style.font.as_ref().or(font);
                let logical_line_height = style.line_height.or(text.line_height);
                let style_size = style.size.unwrap_or(text.font_size);
                let (_, _, line_height) =
                    validate_style_metrics(object_font, style_size, logical_line_height)?;
                let source_size = object.icon.source_size();
                let ratio = source_size.x as f32 / source_size.y as f32;
                let width = height * ratio;
                if !height.is_finite()
                    || height <= 0.0
                    || !ratio.is_finite()
                    || ratio <= 0.0
                    || !width.is_finite()
                    || width <= 0.0
                {
                    return Err("Text icon has an invalid logical size".to_string());
                }
                Ok(shaping::ResolvedInlineObject {
                    icon: object.icon,
                    size: vec2(width, height),
                    align: object.options.align,
                    line_height,
                    style: object.style,
                    source: object.source,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut seen = vec![0_u8; objects.len()];
        let mut line_top = 0.0;
        let mut content_width = 0.0_f32;
        let mut effect_atoms = vec![Vec::new(); markup.effects.len()];
        let mut line_base = 0_usize;

        for buffer_line in &self.buffer.lines {
            let line_text = buffer_line.text();
            let current_line_base = line_base;
            line_base = line_base
                .checked_add(line_text.len())
                .and_then(|base| base.checked_add(buffer_line.ending().as_str().len()))
                .ok_or_else(|| "Text shaping coordinate overflowed".to_string())?;
            let Some(shape) = buffer_line.shape_opt() else {
                continue;
            };
            let has_bidi_controls = line_text.chars().any(shaping::is_bidi_formatting_control);
            let shape = if objects.is_empty() && !has_bidi_controls {
                std::borrow::Cow::Borrowed(shape)
            } else {
                let mut shape = shape.clone();
                shaping::patch_shape(line_text, &mut shape, &markup.spans, &objects, &mut seen)?;
                std::borrow::Cow::Owned(shape)
            };
            let layout_lines = shape.layout(
                font_size,
                text.wrap_width,
                Wrap::WordOrGlyph,
                None,
                None,
                Hinting::Disabled,
            );
            if layout_lines.is_empty() {
                let height = buffer_line
                    .attrs_list()
                    .defaults()
                    .metrics_opt
                    .map(Metrics::from)
                    .map(|metrics| metrics.line_height.max(metrics.font_size))
                    .unwrap_or(base_line_height);
                let visual = layout.atoms.atom_count();
                layout.atoms.push_line(LineGeometry {
                    top: line_top,
                    width: 0.0,
                    height,
                    baseline: line_top,
                    ascent: 0.0,
                    descent: 0.0,
                    visual_atoms: visual..visual,
                    x_offset: 0.0,
                    rtl: false,
                });
                layout.lines.push(rich::RichTextLine {
                    offset_y: line_top,
                    size: vec2(0.0, height),
                });
                line_top += height;
                continue;
            }
            for layout_line in layout_lines {
                let line_index = layout.lines.len();
                let visual_start = layout.atoms.atom_count();
                let min_x = layout_line
                    .glyphs
                    .iter()
                    .filter(|glyph| {
                        !shaping::is_bidi_control_cluster(line_text, glyph.start, glyph.end)
                    })
                    .flat_map(|glyph| [glyph.x, glyph.x + glyph.w])
                    .reduce(f32::min)
                    .unwrap_or(0.0);
                let mut baseline_icon = 0.0_f32;
                let mut non_baseline_icon = 0.0_f32;
                for icon in layout_line
                    .glyphs
                    .iter()
                    .filter_map(|glyph| glyph.metadata.checked_sub(1))
                    .filter_map(|index| markup.spans.get(index))
                    .filter_map(|span| match span.owner {
                        shaping::ShapingOwner::Object(index) => Some(index),
                        _ => None,
                    })
                    .filter_map(|index| objects.get(index))
                {
                    match icon.align {
                        rich::TextIconAlign::Baseline => {
                            baseline_icon = baseline_icon.max(icon.size.y)
                        }
                        _ => non_baseline_icon = non_baseline_icon.max(icon.size.y),
                    }
                }
                let tallest_run = layout_line
                    .glyphs
                    .iter()
                    .filter_map(|glyph| glyph.metadata.checked_sub(1))
                    .filter_map(|index| span_profiles.get(index))
                    .map(|profile| profile.2)
                    .fold(base_line_height, f32::max);
                let text_ascent = layout_line.max_ascent;
                let text_descent = layout_line.max_descent;
                let base_ascent = text_ascent.max(baseline_icon);
                let content_height = base_ascent + text_descent;
                let height = tallest_run.max(content_height).max(non_baseline_icon);
                let baseline = line_top + (height - content_height) * 0.5 + base_ascent;
                let mut content_top = baseline - text_ascent;
                let mut content_bottom = baseline + text_descent;
                for (glyph_index, glyph) in layout_line.glyphs.iter().enumerate() {
                    if shaping::is_bidi_control_cluster(line_text, glyph.start, glyph.end) {
                        continue;
                    }
                    let span_index = glyph
                        .metadata
                        .checked_sub(1)
                        .ok_or_else(|| "Text glyph is missing semantic metadata".to_string())?;
                    let shaping_range =
                        current_line_base + glyph.start..current_line_base + glyph.end;
                    let mapped = shaping::map_range(
                        &markup.map,
                        &markup.spans,
                        shaping_range.clone(),
                        diagnostics,
                    )?;
                    if matches!(
                        mapped.owner,
                        shaping::ShapingOwner::WrapHint | shaping::ShapingOwner::FormattingControl
                    ) {
                        continue;
                    }
                    let span_index = if matches!(mapped.owner, shaping::ShapingOwner::Text) {
                        markup
                            .spans
                            .iter()
                            .position(|span| {
                                matches!(span.owner, shaping::ShapingOwner::Text)
                                    && span.semantic.contains(&mapped.semantic.start)
                            })
                            .unwrap_or(span_index)
                    } else {
                        span_index
                    };
                    let span = markup
                        .spans
                        .get(span_index)
                        .ok_or_else(|| "Text glyph metadata is out of bounds".to_string())?;
                    let source_range = mapped.source;
                    let semantic_range = mapped.semantic;
                    let (pixelated, strike_scale, _) = span_profiles
                        .get(span_index)
                        .copied()
                        .ok_or_else(|| "Text glyph style is out of bounds".to_string())?;
                    let decoration = layout_line
                        .decorations
                        .iter()
                        .find(|decoration| decoration.glyph_range.contains(&glyph_index));
                    if let shaping::ShapingOwner::Object(index) = mapped.owner {
                        let icon = objects
                            .get(index)
                            .ok_or_else(|| "Text icon metadata is out of bounds".to_string())?;
                        if source_range != icon.source {
                            return Err("Text icon source identity changed during shaping".into());
                        }
                        let y = match icon.align {
                            rich::TextIconAlign::Middle => line_top + (height - icon.size.y) * 0.5,
                            rich::TextIconAlign::Baseline => baseline - icon.size.y,
                            rich::TextIconAlign::Top => line_top,
                            rich::TextIconAlign::Bottom => line_top + height - icon.size.y,
                        };
                        content_top = content_top.min(y);
                        content_bottom = content_bottom.max(y + icon.size.y);
                        let atom = layout.atoms.push(NewAtom {
                            kind: AtomKind::Icon,
                            source: source_range,
                            semantic: semantic_range,
                            shaping: shaping_range,
                            line: line_index,
                            style: icon.style,
                            bidi_level: glyph.level.number(),
                            advance: glyph.w,
                            bounds: LogicalBounds {
                                x: glyph.x - min_x,
                                y,
                                width: icon.size.x,
                                height: icon.size.y,
                            },
                            color: span.color,
                        })?;
                        record_effect_atoms(&mut effect_atoms, &span.effects, atom)?;
                        layout.plan.push(RenderItem::Icon(PlacedIcon {
                            atom,
                            icon: icon.icon.clone(),
                            pos: vec2(glyph.x - min_x, y),
                            size: icon.size,
                        }));
                        push_decorations(
                            &mut layout.plan,
                            atom,
                            glyph.x - min_x,
                            icon.size.x,
                            baseline,
                            glyph.font_size,
                            span.underline,
                            span.strikethrough,
                            pixelated,
                            decoration,
                        );
                    } else {
                        let kind = if line_text
                            .get(glyph.start..glyph.end)
                            .is_some_and(|cluster| cluster.chars().all(char::is_whitespace))
                        {
                            AtomKind::Space
                        } else {
                            AtomKind::Text
                        };
                        let atom = layout.atoms.push(NewAtom {
                            kind,
                            source: source_range,
                            semantic: semantic_range,
                            shaping: shaping_range,
                            line: line_index,
                            style: span.style,
                            bidi_level: glyph.level.number(),
                            advance: glyph.w,
                            bounds: LogicalBounds {
                                x: glyph.x - min_x,
                                y: line_top,
                                width: glyph.w,
                                height,
                            },
                            color: span.color,
                        })?;
                        record_effect_atoms(&mut effect_atoms, &span.effects, atom)?;
                        layout.plan.push(RenderItem::Glyph(PlacedGlyph {
                            atom,
                            glyph: glyph.clone(),
                            origin: vec2(-min_x, baseline),
                            pixelated,
                            strike_scale,
                        }));
                        push_decorations(
                            &mut layout.plan,
                            atom,
                            glyph.x - min_x,
                            glyph.w,
                            baseline,
                            glyph.font_size,
                            span.underline,
                            span.strikethrough,
                            pixelated,
                            decoration,
                        );
                    }
                }
                let size = vec2(layout_line.w, height);
                content_width = content_width.max(size.x);
                layout.atoms.push_line(LineGeometry {
                    top: line_top,
                    width: size.x,
                    height,
                    baseline,
                    ascent: baseline - content_top,
                    descent: content_bottom - baseline,
                    visual_atoms: visual_start..layout.atoms.atom_count(),
                    x_offset: 0.0,
                    rtl: layout_line
                        .glyphs
                        .first()
                        .is_some_and(|glyph| glyph.level.is_rtl()),
                });
                layout.lines.push(rich::RichTextLine {
                    offset_y: line_top,
                    size,
                });
                line_top += height;
            }
        }
        if seen.iter().any(|count| *count != 1) {
            return Err("Text icon placeholder did not produce exactly one glyph".into());
        }
        validate_finite(content_width, "Text layout width")?;
        validate_finite(line_top, "Text layout height")?;
        for (line_index, line) in layout.lines.iter().enumerate() {
            let offset = match text.h_align {
                HAlign::Left => 0.0,
                HAlign::Center => (content_width - line.size.x) * 0.5,
                HAlign::Right => content_width - line.size.x,
            };
            layout.plan.offset_line(&layout.atoms, line_index, offset);
            layout.atoms.set_line_offset(line_index, offset)?;
        }
        layout.atoms.finish(layout.semantic_text.len())?;
        for (callback, atoms) in markup.effects.into_iter().zip(effect_atoms) {
            let Some(first) = atoms.first().and_then(|id| layout.atoms.logical_index(*id)) else {
                continue;
            };
            let Some(last) = atoms.last().and_then(|id| layout.atoms.logical_index(*id)) else {
                continue;
            };
            let end = last
                .checked_add(1)
                .ok_or_else(|| "Text effect atom range overflowed".to_string())?;
            if end - first != atoms.len() {
                return Err("Text effect occurrence is not contiguous".into());
            }
            let callback_index = match layout
                .effect_callbacks
                .iter()
                .position(|stored| std::sync::Arc::ptr_eq(stored, &callback))
            {
                Some(index) => index,
                None => {
                    let index = layout.effect_callbacks.len();
                    layout.effect_callbacks.push(callback);
                    index
                }
            };
            layout.effects.push(effect::EffectOccurrence {
                callback: callback_index,
                atoms: first..end,
            });
        }
        let outline_pad = f32::from(text.outline_width) * 2.0;
        let size = vec2(content_width + outline_pad, line_top + outline_pad);
        validate_finite(size.x, "Text layout width")?;
        validate_finite(size.y, "Text layout height")?;
        layout.size = size;
        layout.outline_width = text.outline_width;
        Ok(())
    }

    pub(crate) fn automatic_resolution(scale: f32) -> f32 {
        const QUARTER_OCTAVE: f32 = 1.189_207_1;
        const SQRT_2: f32 = std::f32::consts::SQRT_2;
        const BUCKETS: [f32; 9] = [
            1.0,
            QUARTER_OCTAVE,
            SQRT_2,
            QUARTER_OCTAVE * SQRT_2,
            2.0,
            2.0 * QUARTER_OCTAVE,
            2.0 * SQRT_2,
            2.0 * QUARTER_OCTAVE * SQRT_2,
            4.0,
        ];
        BUCKETS
            .into_iter()
            .find(|bucket| *bucket >= scale)
            .unwrap_or(4.0)
    }

    pub(crate) fn prepare_layout(
        &mut self,
        layout: &TextLayout,
        resolution: f32,
        scratch: &mut TextPrepareScratch,
        quads: &mut Vec<QuadData>,
    ) -> Result<(), String> {
        validate_positive(resolution, "Text resolution")?;
        let items = &mut scratch.items;
        items.clear();
        items.reserve(layout.plan.len());
        for (index, item) in layout.plan.iter().enumerate() {
            let atom = layout
                .atoms
                .logical_index(item.atom())
                .ok_or_else(|| "Text render item has an invalid atom".to_string())?;
            let state = scratch
                .states
                .get(atom)
                .copied()
                .ok_or_else(|| "Text prepared atom is missing".to_string())?;
            if state.hidden {
                continue;
            }
            let color = state.color.with_alpha(state.color.a * state.alpha);
            match item {
                RenderItem::Glyph(glyph) => {
                    let scale = resolution * glyph.strike_scale;
                    validate_positive(scale, "Text effective resolution")?;
                    let physical_outline = (f32::from(layout.outline_width) * scale).ceil();
                    if physical_outline > u16::MAX as f32 {
                        return Err("Text outline width exceeds glyph cache limits".into());
                    }
                    let physical = glyph.glyph.physical((0.0, 0.0), scale);
                    items.push(RasterItem::Glyph {
                        atom,
                        key: physical.cache_key,
                        pos: glyph.origin
                            + vec2(physical.x as f32, physical.y as f32) / scale
                            + Vec2::splat(f32::from(layout.outline_width)),
                        scale,
                        outline_radius: physical_outline as u16,
                        color,
                        pixelated: glyph.pixelated,
                    });
                }
                RenderItem::Icon(_) => items.push(RasterItem::Icon { index, color, atom }),
                RenderItem::Solid(solid) => {
                    let min_size = Vec2::splat(1.0 / resolution);
                    let mut pos = solid.pos;
                    let mut size = solid.size.max(min_size);
                    if solid.pixelated {
                        pos = (pos * resolution).round() / resolution;
                        size = (size * resolution).round().max(Vec2::ONE) / resolution;
                    }
                    items.push(RasterItem::Solid {
                        atom,
                        pos,
                        size,
                        color,
                        pixelated: solid.pixelated,
                    });
                }
            }
        }

        self.ensure_raster(layout, items)?;
        self.resolve_raster(layout, items, quads);
        Ok(())
    }

    fn ensure_raster(&mut self, layout: &TextLayout, items: &[RasterItem]) -> Result<(), String> {
        let mut resets = [false; TextAtlas::COUNT];
        loop {
            match self.ensure_sources(layout, items)? {
                ProcessResult::Ready => return self.flush_pending_icons(),
                ProcessResult::Full(atlas) => {
                    if self.grow_atlas(atlas)? {
                        continue;
                    }
                    if resets[atlas.index()] {
                        return Err(format!(
                            "Text {} atlas cannot fit this text after a maximum-size reset",
                            atlas.name()
                        ));
                    }
                    self.reset_atlas(atlas)?;
                    resets[atlas.index()] = true;
                }
            }
        }
    }

    fn resolve_raster(&self, layout: &TextLayout, items: &[RasterItem], quads: &mut Vec<QuadData>) {
        quads.clear();
        quads.reserve(items.len());
        for item in items {
            match item {
                RasterItem::Glyph { .. } => self.resolve_glyph(layout, item, quads),
                RasterItem::Icon { index, color, atom } => {
                    let Some(RenderItem::Icon(icon)) = layout.plan.get(*index) else {
                        continue;
                    };
                    self.resolve_icon(
                        icon,
                        Vec2::splat(f32::from(layout.outline_width)),
                        *color,
                        *atom,
                        quads,
                    );
                }
                RasterItem::Solid {
                    atom,
                    pos,
                    size,
                    color,
                    pixelated,
                } => quads.push(QuadData {
                    atom: *atom,
                    xy: *pos + Vec2::splat(f32::from(layout.outline_width)),
                    size: *size,
                    uvs1: Vec2::ZERO,
                    uvs2: Vec2::ZERO,
                    source: TextSource::Solid,
                    color: *color,
                    pixelated: *pixelated,
                    outline: None,
                }),
            }
        }
    }

    fn resolve_glyph(&self, layout: &TextLayout, item: &RasterItem, quads: &mut Vec<QuadData>) {
        let RasterItem::Glyph {
            atom,
            key,
            pos,
            scale,
            outline_radius,
            color,
            pixelated,
        } = item
        else {
            return;
        };
        let key = *key;
        let pos = *pos;
        let scale = *scale;
        let outline_radius = *outline_radius;
        let color = *color;
        let pixelated = *pixelated;
        let normal_key = GlyphCacheKey {
            key,
            outline: 0,
            pixelated,
        };
        let Some(info) = self.cache.get(&normal_key) else {
            return;
        };
        let Some(atlas) = info.atlas else { return };
        let atlas_size = self.atlas(atlas).texture.size();
        let atlas_glyph_size = info.size.as_vec2();
        let mut size = atlas_glyph_size / scale;
        if pixelated {
            size = size.round();
        }
        let mut xy = pos + info.pos.as_vec2() / scale;
        if pixelated {
            xy = xy.round();
        }
        let outline = if outline_radius > 0 {
            let outline_key = GlyphCacheKey {
                key,
                outline: outline_radius,
                pixelated,
            };
            self.cache.get(&outline_key).map(|info| {
                let texture_size = self.atlas(TextAtlas::mask(pixelated)).texture.size();
                let outline_size = info.size.as_vec2();
                let logical_width = f32::from(layout.outline_width);
                let surplus = (f32::from(outline_radius) - logical_width * scale).max(0.0);
                OutlineQuad {
                    xy: xy - Vec2::splat(logical_width),
                    size: size + Vec2::splat(logical_width * 2.0),
                    uvs1: (info.atlas_pos + surplus) / texture_size,
                    uvs2: (info.atlas_pos + outline_size - surplus) / texture_size,
                    source: TextSource::mask(pixelated),
                }
            })
        } else {
            None
        };
        quads.push(QuadData {
            atom: *atom,
            xy,
            size,
            uvs1: info.atlas_pos / atlas_size,
            uvs2: (info.atlas_pos + atlas_glyph_size) / atlas_size,
            source: atlas.source(pixelated),
            color,
            pixelated,
            outline,
        });
    }

    fn resolve_icon(
        &self,
        icon: &PlacedIcon,
        offset: Vec2,
        color: Color,
        atom: usize,
        quads: &mut Vec<QuadData>,
    ) {
        let key = IconCacheKey::from(&icon.icon);
        let Some(info) = self.icon_cache.get(&key) else {
            return;
        };
        let texture_size = self.atlas(info.atlas).texture.size();
        let source_size = icon.icon.source_size().as_vec2();
        let inner = info.outer_pos + Vec2::ONE;
        quads.push(QuadData {
            atom,
            xy: icon.pos + offset,
            size: icon.size,
            uvs1: inner / texture_size,
            uvs2: (inner + source_size) / texture_size,
            source: info.atlas.source(false),
            color,
            pixelated: false,
            outline: None,
        });
    }

    fn atlas(&self, atlas: TextAtlas) -> &AtlasData {
        &self.atlases[atlas.index()]
    }

    fn atlas_mut(&mut self, atlas: TextAtlas) -> &mut AtlasData {
        &mut self.atlases[atlas.index()]
    }

    pub(crate) fn mask_atlas_texture(&self) -> Texture {
        self.atlas(TextAtlas::MaskLinear).texture.texture().clone()
    }

    pub(crate) fn color_atlas_texture(&self) -> Texture {
        self.atlas(TextAtlas::RgbaLinear).texture.texture().clone()
    }

    fn grow_atlas(&mut self, atlas: TextAtlas) -> Result<bool, String> {
        let Some(mut generation) = self.atlas(atlas).grown_generation()? else {
            return Ok(false);
        };
        self.restore_into(atlas, &generation.texture)?;
        self.atlas_mut(atlas).commit(&mut generation);
        self.bind_group = None;
        Ok(true)
    }

    fn reset_atlas(&mut self, atlas: TextAtlas) -> Result<(), String> {
        let mut generation = self.atlas(atlas).fresh_generation()?;
        self.atlas_mut(atlas).commit(&mut generation);
        self.cache.retain(|_, glyph| glyph.atlas != Some(atlas));
        self.icon_cache.retain(|_, icon| icon.atlas != atlas);
        self.pending_icons.retain(|key, _| key.atlas != atlas);
        self.bind_group = None;
        Ok(())
    }

    fn restore_into(&mut self, atlas: TextAtlas, texture: &RenderTexture) -> Result<(), String> {
        let glyphs: Vec<_> = self
            .cache
            .iter()
            .filter(|(_, glyph)| glyph.atlas == Some(atlas))
            .map(|(key, glyph)| (*key, glyph.clone()))
            .collect();
        for (key, glyph) in glyphs {
            let image = self
                .swash
                .get_image_uncached(&mut self.font_system, key.key)
                .ok_or_else(|| "Cannot restore a cached text glyph".to_string())?;
            let offset = glyph.atlas_pos.as_uvec2();
            if key.outline > 0 {
                let outline = outline_info(
                    image.placement.width,
                    image.placement.height,
                    image.placement.left,
                    image.placement.top,
                    key.outline,
                    self.atlas(atlas),
                )?;
                expanded_mask(&mut self.temp_outline_buff, &image.data, &outline)?;
                upload_texture(texture, outline.size, offset, &self.temp_outline_buff)?;
            } else {
                upload_texture(
                    texture,
                    uvec2(glyph.size.x as _, glyph.size.y as _),
                    offset,
                    &image.data,
                )?;
            }
        }

        let restored: Vec<_> = self
            .icon_cache
            .iter()
            .filter(|(_, icon)| icon.atlas == atlas)
            .map(|(key, icon)| {
                let revision = icon.sprite.texture().revision();
                (
                    *key,
                    revision,
                    IconBake {
                        sprite: icon.sprite.clone(),
                        frame: key.frame,
                        outer_pos: icon.outer_pos,
                    },
                )
            })
            .collect();
        let bakes: Vec<_> = restored.iter().map(|(_, _, bake)| bake.clone()).collect();
        self.icon_baker.bake(texture, &bakes)?;
        for (key, revision, _) in restored {
            if let Some(icon) = self.icon_cache.get_mut(&key) {
                icon.baked_revision = Some(revision);
            }
            self.pending_icons.remove(&key);
        }
        Ok(())
    }

    fn ensure_sources(
        &mut self,
        layout: &TextLayout,
        items: &[RasterItem],
    ) -> Result<ProcessResult, String> {
        for item in items {
            match item {
                RasterItem::Glyph {
                    key,
                    outline_radius,
                    ..
                } => {
                    if let Some(full) =
                        self.ensure_glyph(*key, *outline_radius, item.pixelated())?
                    {
                        return Ok(ProcessResult::Full(full));
                    }
                }
                RasterItem::Icon { index, .. } => {
                    let Some(RenderItem::Icon(icon)) = layout.plan.get(*index) else {
                        continue;
                    };
                    if let Some(full) = self.ensure_icon(&icon.icon)? {
                        return Ok(ProcessResult::Full(full));
                    }
                }
                RasterItem::Solid { .. } => {}
            }
        }
        Ok(ProcessResult::Ready)
    }

    fn ensure_glyph(
        &mut self,
        key: CacheKey,
        outline_width: u16,
        pixelated: bool,
    ) -> Result<Option<TextAtlas>, String> {
        let normal_key = GlyphCacheKey {
            key,
            outline: 0,
            pixelated,
        };
        let normal_cached = self.cache.contains_key(&normal_key);
        if outline_width > 0 {
            let outline_key = GlyphCacheKey {
                key,
                outline: outline_width,
                pixelated,
            };
            if !self.cache.contains_key(&outline_key)
                && let Some(image) = self.swash.get_image_uncached(&mut self.font_system, key)
                && image.placement.width > 0
                && image.placement.height > 0
                && image.content == SwashContent::Mask
            {
                let outline = outline_info(
                    image.placement.width,
                    image.placement.height,
                    image.placement.left,
                    image.placement.top,
                    outline_width,
                    self.atlas(TextAtlas::mask(pixelated)),
                )?;
                expanded_mask(&mut self.temp_outline_buff, &image.data, &outline)?;
                let mask = TextAtlas::mask(pixelated);
                let atlas = &mut self.atlases[mask.index()];
                let Some(atlas_pos) = atlas.store(outline.size, &self.temp_outline_buff)? else {
                    return Ok(Some(mask));
                };
                self.cache.insert(
                    outline_key,
                    GlyphInfo {
                        pos: outline.pos,
                        size: outline.cache_size,
                        atlas_pos,
                        atlas: Some(mask),
                    },
                );
            }
        }
        if normal_cached {
            return Ok(None);
        }
        let Some(image) = self.swash.get_image_uncached(&mut self.font_system, key) else {
            return Ok(None);
        };
        if image.placement.width == 0 || image.placement.height == 0 {
            self.cache.insert(
                normal_key,
                GlyphInfo {
                    pos: Pos::new(0, 0),
                    size: Pos::new(0, 0),
                    atlas_pos: Default::default(),
                    atlas: None,
                },
            );
            return Ok(None);
        }
        let atlas = match image.content {
            SwashContent::Mask => TextAtlas::mask(pixelated),
            SwashContent::Color => {
                if pixelated {
                    TextAtlas::RgbaNearest
                } else {
                    TextAtlas::RgbaLinear
                }
            }
            SwashContent::SubpixelMask => return Ok(None),
        };
        let Some(atlas_pos) = self.atlas_mut(atlas).store(
            uvec2(image.placement.width, image.placement.height),
            &image.data,
        )?
        else {
            return Ok(Some(atlas));
        };
        self.cache.insert(
            normal_key,
            GlyphInfo {
                pos: Pos::new(image.placement.left as _, -image.placement.top as _),
                size: Pos::new(image.placement.width as _, image.placement.height as _),
                atlas_pos,
                atlas: Some(atlas),
            },
        );
        Ok(None)
    }

    fn ensure_icon(&mut self, icon: &RegisteredIcon) -> Result<Option<TextAtlas>, String> {
        let key = IconCacheKey::from(icon);
        let revision = icon.sprite.texture().revision();
        if let Some(cached) = self.icon_cache.get(&key) {
            if cached.baked_revision != Some(revision) {
                self.pending_icons.insert(key, revision);
            }
            return Ok(None);
        }

        let size = padded_icon_size(icon.source_size())?;
        let Some(outer_pos) = self.atlas_mut(icon.atlas).allocate(size)? else {
            return Ok(Some(icon.atlas));
        };
        self.icon_cache.insert(
            key,
            IconInfo {
                sprite: icon.sprite.clone(),
                outer_pos,
                atlas: icon.atlas,
                baked_revision: None,
            },
        );
        self.pending_icons.insert(key, revision);
        Ok(None)
    }

    fn flush_pending_icons(&mut self) -> Result<(), String> {
        for atlas in [TextAtlas::RgbaLinear, TextAtlas::RgbaNearest] {
            let pending: Vec<_> = self
                .pending_icons
                .iter()
                .filter(|(key, _)| key.atlas == atlas)
                .filter_map(|(key, revision)| {
                    self.icon_cache.get(key).map(|icon| {
                        (
                            *key,
                            *revision,
                            IconBake {
                                sprite: icon.sprite.clone(),
                                frame: key.frame,
                                outer_pos: icon.outer_pos,
                            },
                        )
                    })
                })
                .collect();
            if pending.is_empty() {
                continue;
            }

            let target = self.atlas(atlas).texture.clone();
            let bakes: Vec<_> = pending.iter().map(|(_, _, bake)| bake.clone()).collect();
            self.icon_baker.bake(&target, &bakes)?;
            for (key, revision, _) in pending {
                if let Some(icon) = self.icon_cache.get_mut(&key) {
                    icon.baked_revision = Some(revision);
                }
                self.pending_icons.remove(&key);
            }
        }
        Ok(())
    }
}

fn font_attrs(font: Option<&Font>) -> Attrs<'_> {
    match font {
        Some(font) => Attrs::new()
            .family(Family::Name(&font.family))
            .weight(font.weight)
            .style(font.style)
            .stretch(font.stretch),
        None => Attrs::new(),
    }
}

fn validate_style_metrics(
    font: Option<&Font>,
    logical_size: f32,
    logical_line_height: Option<f32>,
) -> Result<(f32, f32, f32), String> {
    let (pixelated, ppem, res_ppem, line_height_pem) = font
        .map(|font| {
            (
                font.is_pixelated(),
                font.px_per_em,
                font.res_ppem,
                font.line_height_pem,
            )
        })
        .unwrap_or((false, 1.0, 1.0, 1.0));
    let info = TextInfo {
        font,
        text: "",
        wrap_width: None,
        font_size: logical_size,
        line_height: logical_line_height,
        h_align: HAlign::Left,
        color_tags: false,
        default_color: Color::WHITE,
        outline_width: 0,
        strict_metrics: true,
    };
    validate_layout_metrics(&info, pixelated, ppem, res_ppem, line_height_pem)
}

fn validate_layout_metrics(
    text: &TextInfo,
    pixelated: bool,
    ppem: f32,
    res_ppem: f32,
    line_height_pem: f32,
) -> Result<(f32, f32, f32), String> {
    if text.strict_metrics {
        validate_positive(text.font_size, "Text size")?;
        if let Some(height) = text.line_height {
            validate_positive(height, "Text line height")?;
        }
        if let Some(width) = text.wrap_width {
            validate_positive(width, "Text maximum width")?;
        }
    }
    validate_finite(text.font_size, "Text size")?;
    if let Some(height) = text.line_height {
        validate_finite(height, "Text line height")?;
    }
    if let Some(width) = text.wrap_width {
        validate_finite(width, "Text maximum width")?;
    }
    validate_positive(ppem, "Text font pixel scale")?;
    validate_positive(line_height_pem, "Text font line-height scale")?;

    let font_size = text.font_size * ppem;
    validate_positive(font_size, "Text effective font size")?;
    let strike_scale = if pixelated {
        validate_positive(res_ppem, "Text pixel font resolution")?;
        if res_ppem > usize::MAX as f32 {
            return Err("Text pixel font resolution is out of range".into());
        }
        let base = res_ppem as usize;
        if base == 0 {
            return Err("Text pixel font resolution is out of range".into());
        }
        let rounded_size = font_size.round();
        if rounded_size > usize::MAX as f32 {
            return Err("Text effective font size is out of range".into());
        }
        let snapped_size = (closest_multiple_of(rounded_size as usize, base) as f32).max(res_ppem);
        validate_positive(snapped_size, "Text snapped pixel font size")?;
        snapped_size / font_size
    } else {
        1.0
    };
    validate_positive(strike_scale, "Text font strike scale")?;

    let line_height = text.line_height.unwrap_or(font_size * line_height_pem);
    if text.line_height.is_some() {
        validate_finite(line_height, "Text effective line height")?;
    } else {
        validate_positive(line_height, "Text effective line height")?;
    }
    Ok((font_size, strike_scale, line_height))
}

fn validate_finite(value: f32, name: &str) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("{name} must be finite"))
    }
}

fn validate_positive(value: f32, name: &str) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{name} must be finite and greater than zero"))
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct IconCacheKey {
    texture: TextureId,
    frame: PixelRect,
    atlas: TextAtlas,
}

impl From<&RegisteredIcon> for IconCacheKey {
    fn from(icon: &RegisteredIcon) -> Self {
        Self {
            texture: icon.sprite.texture().id(),
            frame: icon.frame,
            atlas: icon.atlas,
        }
    }
}

#[derive(Clone)]
struct IconInfo {
    sprite: crate::Sprite,
    outer_pos: Vec2,
    atlas: TextAtlas,
    baked_revision: Option<u32>,
}

fn padded_icon_size(source: UVec2) -> Result<UVec2, String> {
    let width = source
        .x
        .checked_add(2)
        .ok_or_else(|| "Text icon width overflowed".to_string())?;
    let height = source
        .y
        .checked_add(2)
        .ok_or_else(|| "Text icon height overflowed".to_string())?;
    Ok(uvec2(width, height))
}

#[derive(Copy, Clone, Debug)]
enum ProcessResult {
    Ready,
    Full(TextAtlas),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum TextAtlas {
    MaskLinear,
    MaskNearest,
    RgbaLinear,
    RgbaNearest,
}

impl TextAtlas {
    const COUNT: usize = 4;

    fn mask(pixelated: bool) -> Self {
        if pixelated {
            Self::MaskNearest
        } else {
            Self::MaskLinear
        }
    }

    fn index(self) -> usize {
        match self {
            Self::MaskLinear => 0,
            Self::MaskNearest => 1,
            Self::RgbaLinear => 2,
            Self::RgbaNearest => 3,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::MaskLinear => "linear mask",
            Self::MaskNearest => "nearest mask",
            Self::RgbaLinear => "linear RGBA",
            Self::RgbaNearest => "nearest RGBA",
        }
    }

    fn source(self, _pixelated: bool) -> TextSource {
        match self {
            Self::MaskLinear => TextSource::MaskLinear,
            Self::MaskNearest => TextSource::MaskNearest,
            Self::RgbaLinear => TextSource::RgbaLinear,
            Self::RgbaNearest => TextSource::RgbaNearest,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum TextSource {
    MaskLinear,
    MaskNearest,
    RgbaLinear,
    RgbaNearest,
    RgbaMaskLinear,
    RgbaMaskNearest,
    Solid,
}

impl TextSource {
    fn mask(pixelated: bool) -> Self {
        if pixelated {
            Self::MaskNearest
        } else {
            Self::MaskLinear
        }
    }

    pub(crate) fn as_mask(self) -> Self {
        match self {
            Self::RgbaLinear => Self::RgbaMaskLinear,
            Self::RgbaNearest => Self::RgbaMaskNearest,
            source => source,
        }
    }

    pub(crate) fn selector(self) -> f32 {
        match self {
            Self::MaskLinear => 0.0,
            Self::MaskNearest => 1.0,
            Self::RgbaLinear => 2.0,
            Self::RgbaNearest => 3.0,
            Self::RgbaMaskLinear => 4.0,
            Self::RgbaMaskNearest => 5.0,
            Self::Solid => 6.0,
        }
    }
}

pub(crate) struct AtlasData {
    pub(crate) texture: RenderTexture,
    allocator: BucketedAtlasAllocator,
    max_texture_size: u32,
    current_size: u32,
    label: &'static str,
}

struct AtlasGeneration {
    texture: RenderTexture,
    allocator: BucketedAtlasAllocator,
    size: u32,
}

impl AtlasData {
    fn new(
        label: &'static str,
        format: TextureFormat,
        max_texture_size: u32,
    ) -> Result<Self, String> {
        let max_texture_size = max_texture_size.min(u32::from(u16::MAX) - 1);
        let current_size = DEFAULT_TEXTURE_SIZE.min(max_texture_size);
        if current_size == 0 {
            return Err("Text atlas maximum texture size is zero".into());
        }
        Ok(Self {
            texture: Self::create_texture(label, format, current_size)?,
            allocator: BucketedAtlasAllocator::new(size2(current_size as _, current_size as _)),
            max_texture_size,
            current_size,
            label,
        })
    }

    fn create_texture(
        label: &'static str,
        format: TextureFormat,
        size: u32,
    ) -> Result<RenderTexture, String> {
        gfx::create_render_texture()
            .with_label(label)
            .with_size(size, size)
            .with_format(format)
            .build()
    }

    fn preflight(&self, size: UVec2) -> Result<(i32, i32), String> {
        let width = size
            .x
            .checked_add(ATLAS_PIXEL_OFFSET)
            .ok_or_else(|| "Text atlas allocation width overflowed".to_string())?;
        let height = size
            .y
            .checked_add(ATLAS_PIXEL_OFFSET)
            .ok_or_else(|| "Text atlas allocation height overflowed".to_string())?;
        if width > self.max_texture_size || height > self.max_texture_size {
            return Err(format!(
                "Text atlas entry {width}x{height} exceeds the {}x{} atlas limit",
                self.max_texture_size, self.max_texture_size
            ));
        }
        let width = i32::try_from(width)
            .map_err(|_| "Text atlas allocation width exceeds allocator limits".to_string())?;
        let height = i32::try_from(height)
            .map_err(|_| "Text atlas allocation height exceeds allocator limits".to_string())?;
        Ok((width, height))
    }

    fn allocate(&mut self, size: UVec2) -> Result<Option<Vec2>, String> {
        let (width, height) = self.preflight(size)?;
        let Some(alloc) = self.allocator.allocate(size2(width, height)) else {
            return Ok(None);
        };
        let offset = vec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _);
        Ok(Some(offset))
    }

    fn store(&mut self, size: UVec2, data: &[u8]) -> Result<Option<Vec2>, String> {
        let (width, height) = self.preflight(size)?;
        let Some(alloc) = self.allocator.allocate(size2(width, height)) else {
            return Ok(None);
        };
        let offset = uvec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _);
        if let Err(error) = upload_texture(&self.texture, size, offset, data) {
            self.allocator.deallocate(alloc.id);
            return Err(error);
        }
        Ok(Some(offset.as_vec2()))
    }

    fn grown_generation(&self) -> Result<Option<AtlasGeneration>, String> {
        if self.current_size >= self.max_texture_size {
            return Ok(None);
        }
        let size = self
            .current_size
            .checked_mul(2)
            .unwrap_or(self.max_texture_size)
            .min(self.max_texture_size);
        let mut allocator = self.allocator.clone();
        allocator.grow(size2(size as _, size as _));
        Ok(Some(AtlasGeneration {
            texture: Self::create_texture(self.label, self.texture.format(), size)?,
            allocator,
            size,
        }))
    }

    fn fresh_generation(&self) -> Result<AtlasGeneration, String> {
        Ok(AtlasGeneration {
            texture: Self::create_texture(self.label, self.texture.format(), self.current_size)?,
            allocator: BucketedAtlasAllocator::new(size2(
                self.current_size as _,
                self.current_size as _,
            )),
            size: self.current_size,
        })
    }

    fn commit(&mut self, generation: &mut AtlasGeneration) {
        std::mem::swap(&mut self.texture, &mut generation.texture);
        std::mem::swap(&mut self.allocator, &mut generation.allocator);
        self.current_size = generation.size;
    }
}

fn upload_texture(
    texture: &Texture,
    size: UVec2,
    offset: UVec2,
    data: &[u8],
) -> Result<(), String> {
    log::trace!("Uploading new glyph to text atlas");
    gfx::write_texture(texture)
        .from_data(data)
        .with_offset(offset)
        .with_size(size)
        .build()
}

#[derive(Clone, Debug)]
struct GlyphInfo {
    pos: Pos<i16>,
    size: Pos<u16>,
    atlas_pos: Vec2,
    atlas: Option<TextAtlas>,
}

#[derive(Copy, Clone, Debug)]
struct Pos<N> {
    x: N,
    y: N,
}

impl<N> Pos<N> {
    pub fn new(x: N, y: N) -> Self {
        Self { x, y }
    }
}

impl Pos<i16> {
    pub fn as_vec2(self) -> Vec2 {
        vec2(self.x as _, self.y as _)
    }
}

impl Pos<u16> {
    pub fn as_vec2(self) -> Vec2 {
        vec2(self.x as _, self.y as _)
    }
}

struct OutlineInfo {
    size: UVec2,
    cache_size: Pos<u16>,
    pos: Pos<i16>,
    source_width: usize,
    source_height: usize,
    radius: usize,
    output_width: usize,
    output_len: usize,
}

fn outline_info(
    source_width: u32,
    source_height: u32,
    left: i32,
    top: i32,
    outline_width: u16,
    atlas: &AtlasData,
) -> Result<OutlineInfo, String> {
    let radius = u32::from(outline_width);
    let padding = radius
        .checked_mul(2)
        .ok_or_else(|| "Text outline dimensions overflowed".to_string())?;
    let width = source_width
        .checked_add(padding)
        .ok_or_else(|| "Text outline dimensions overflowed".to_string())?;
    let height = source_height
        .checked_add(padding)
        .ok_or_else(|| "Text outline dimensions overflowed".to_string())?;
    let size = uvec2(width, height);
    atlas.preflight(size)?;
    let cache_size = Pos::new(
        u16::try_from(width)
            .map_err(|_| "Text outline width exceeds glyph cache limits".to_string())?,
        u16::try_from(height)
            .map_err(|_| "Text outline height exceeds glyph cache limits".to_string())?,
    );

    let radius = i16::try_from(radius)
        .map_err(|_| "Text outline width exceeds glyph position limits".to_string())?;
    let radius_usize = radius as usize;
    let left = i16::try_from(left)
        .map_err(|_| "Text glyph horizontal position exceeds supported limits".to_string())?;
    let top = i16::try_from(top)
        .map_err(|_| "Text glyph vertical position exceeds supported limits".to_string())?;
    let x = left
        .checked_sub(radius)
        .ok_or_else(|| "Text outline horizontal position overflowed".to_string())?;
    let y = top
        .checked_add(radius)
        .and_then(i16::checked_neg)
        .ok_or_else(|| "Text outline vertical position overflowed".to_string())?;

    let source_width = usize::try_from(source_width)
        .map_err(|_| "Text outline source width exceeds platform limits".to_string())?;
    let source_height = usize::try_from(source_height)
        .map_err(|_| "Text outline source height exceeds platform limits".to_string())?;
    let output_width = usize::try_from(width)
        .map_err(|_| "Text outline buffer exceeds platform limits".to_string())?;
    let output_len = usize::try_from(height)
        .ok()
        .and_then(|height| output_width.checked_mul(height))
        .ok_or_else(|| "Text outline buffer exceeds platform limits".to_string())?;
    if output_len > MAX_OUTLINE_BUFFER_BYTES {
        return Err(format!(
            "Text outline bitmap exceeds the {} MiB limit",
            MAX_OUTLINE_BUFFER_BYTES / (1024 * 1024)
        ));
    }

    Ok(OutlineInfo {
        size,
        cache_size,
        pos: Pos::new(x, y),
        source_width,
        source_height,
        radius: radius_usize,
        output_width,
        output_len,
    })
}

fn expanded_mask(dst: &mut Vec<u8>, src: &[u8], outline: &OutlineInfo) -> Result<(), String> {
    let source_len = outline
        .source_width
        .checked_mul(outline.source_height)
        .ok_or_else(|| "Text outline source buffer exceeds platform limits".to_string())?;
    if src.len() < source_len {
        return Err("Text outline source buffer is smaller than its glyph dimensions".into());
    }
    dst.clear();
    dst.try_reserve_exact(outline.output_len)
        .map_err(|_| "Cannot allocate text outline buffer".to_string())?;
    dst.resize(outline.output_len, 0);

    let width = outline.output_width;
    let radius = outline.radius;
    for sy in 0..outline.source_height {
        let src_row = sy * outline.source_width;
        for sx in 0..outline.source_width {
            if src[src_row + sx] == 0 {
                continue;
            }
            let x0 = sx;
            let x1 = sx + radius * 2 + 1;
            for dy in 0..=radius * 2 {
                let row_start = (sy + dy) * width;
                dst[row_start + x0..row_start + x1].fill(255);
            }
        }
    }
    Ok(())
}
