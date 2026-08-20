use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use corelib::gfx::{
    self, BindGroup, Color, RenderPipeline, RenderTexture, Sampler, Texture, TextureFilter,
    TextureFormat, TextureId,
};
use corelib::math::{UVec2, Vec2, uvec2, vec2};
use cosmic_text::fontdb::Source;
use cosmic_text::{
    Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, ShapeLine, Shaping, Stretch, Style,
    SwashCache, SwashContent, Weight, Wrap,
};
use etagere::{BucketedAtlasAllocator, size2};
use markup::MarkupMode;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use std::sync::Arc;

mod icon_baker;
mod markup;
mod rich;

use icon_baker::{IconBake, IconBaker};
use rich::{PixelRect, RegisteredIcon};
pub use rich::{RichTextBuilder, RichTextLayout, RichTextLine, TextIcons, rich_text};
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
    // raw: ID,
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
    items: Vec<LayoutItem>,
    resolution: f32,
    pixelated: bool,
    outline_width: u16,
    outline_radius: f32,
}

impl Default for TextLayout {
    fn default() -> Self {
        Self {
            size: Vec2::ZERO,
            lines: Vec::new(),
            items: Vec::new(),
            resolution: 1.0,
            pixelated: false,
            outline_width: 0,
            outline_radius: 0.0,
        }
    }
}

impl TextLayout {
    fn clear(&mut self) {
        self.size = Vec2::ZERO;
        self.lines.clear();
        self.items.clear();
        self.resolution = 1.0;
        self.pixelated = false;
        self.outline_width = 0;
        self.outline_radius = 0.0;
    }
}

enum LayoutItem {
    Glyph(PlacedGlyph),
    Icon(PlacedIcon),
}

struct PlacedGlyph {
    key: CacheKey,
    pos: Vec2,
    color: Color,
}

struct PlacedIcon {
    icon: RegisteredIcon,
    pos: Vec2,
    size: Vec2,
    color: Color,
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
    pub(crate) resolution: f32,
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
            resolution: 1.0,
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

    pub fn resolution(mut self, scale: f32) -> Self {
        self.info.resolution = scale;
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
}

pub struct TextSystem {
    pub(crate) mask: AtlasData,
    pub(crate) rgba_linear: AtlasData,
    pub(crate) rgba_nearest: AtlasData,
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
    temp_line_items: Vec<std::ops::Range<usize>>,
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

        let mask = AtlasData::new(
            "TextSystem Texture Mask",
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
            mask,
            rgba_linear,
            rgba_nearest,
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
            temp_line_items: Vec::new(),
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
                .with_texture(2, &self.mask.texture)
                .with_texture(3, &self.rgba_linear.texture)
                .with_texture(4, &self.rgba_nearest.texture)
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

        let font = self
            .font_system
            .get_font(raw_id)
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

        let face = self
            .font_system
            .db()
            .face(raw_id)
            .ok_or_else(|| "Invalid font type".to_string())?;

        Ok(Font {
            id: FontId(id),
            // raw: raw_id,
            nearest,
            res_ppem,
            px_per_em,
            line_height_pem,
            family: Arc::new(face.families[0].0.to_string()),
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
        let markup = if text.color_tags {
            markup::parse(
                text.text,
                text.default_color,
                MarkupMode::Colors,
                text.wrap_width.is_some(),
            )
        } else {
            markup::plain(text.text, text.default_color, text.wrap_width.is_some())
        };
        self.layout_markup(text, markup, layout)
    }

    pub(crate) fn layout_markup(
        &mut self,
        text: &TextInfo,
        markup: markup::Markup<'_>,
        layout: &mut TextLayout,
    ) -> Result<(), String> {
        layout.clear();
        self.temp_line_items.clear();
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
        let attrs = match font {
            Some(font) => Attrs::new()
                .family(Family::Name(&font.family))
                .weight(font.weight)
                .style(font.style)
                .stretch(font.stretch),
            None => Attrs::new(),
        };
        let (font_size, resolution, base_line_height) =
            validate_layout_metrics(text, pixelated, ppem, res_ppem, line_height_pem)?;
        self.buffer.set_metrics(
            &mut self.font_system,
            Metrics::new(font_size, base_line_height),
        );
        self.buffer
            .set_size(&mut self.font_system, text.wrap_width, None);

        let spans = markup
            .spans
            .iter()
            .enumerate()
            .map(|(index, span)| (&markup.text[span.range.clone()], attrs.metadata(index + 1)));
        self.buffer
            .set_rich_text(&mut self.font_system, spans, attrs, Shaping::Advanced);
        self.buffer.shape_until_scroll(&mut self.font_system, false);

        let objects: Vec<_> = markup
            .objects
            .into_iter()
            .map(|object| {
                let height = object.height.unwrap_or(text.font_size);
                let source_size = object.icon.source_size();
                let width = height * source_size.x as f32 / source_size.y as f32;
                if !height.is_finite() || height <= 0.0 || !width.is_finite() || width <= 0.0 {
                    return Err("Text icon has an invalid logical size".to_string());
                }
                Ok(PlacedIcon {
                    icon: object.icon,
                    pos: Vec2::ZERO,
                    size: vec2(width, height),
                    color: object.color,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut seen = vec![0_u8; objects.len()];
        let mut line_top = 0.0;
        let mut content_width = 0.0_f32;

        for buffer_line in &self.buffer.lines {
            let Some(shape) = buffer_line.shape_opt().as_ref() else {
                continue;
            };
            let line_text = buffer_line.text();
            let has_bidi_controls = line_text.chars().any(is_bidi_formatting_control);
            let shape = if objects.is_empty() && !has_bidi_controls {
                std::borrow::Cow::Borrowed(shape)
            } else {
                let mut shape = shape.clone();
                patch_shape(line_text, &mut shape, &markup.spans, &objects, &mut seen)?;
                std::borrow::Cow::Owned(shape)
            };
            let layout_lines =
                shape.layout(font_size, text.wrap_width, Wrap::WordOrGlyph, None, None);
            if layout_lines.is_empty() {
                let start = layout.items.len();
                layout.lines.push(rich::RichTextLine {
                    offset_y: line_top,
                    size: vec2(0.0, base_line_height),
                });
                self.temp_line_items.push(start..start);
                line_top += base_line_height;
                continue;
            }
            for layout_line in layout_lines {
                let start = layout.items.len();
                let min_x = layout_line
                    .glyphs
                    .iter()
                    .filter(|glyph| !is_bidi_control_cluster(line_text, glyph.start, glyph.end))
                    .flat_map(|glyph| [glyph.x, glyph.x + glyph.w])
                    .reduce(f32::min)
                    .unwrap_or(0.0);
                let tallest_icon = layout_line
                    .glyphs
                    .iter()
                    .filter_map(|glyph| glyph.metadata.checked_sub(1))
                    .filter_map(|index| markup.spans.get(index))
                    .filter_map(|span| span.object)
                    .filter_map(|index| objects.get(index))
                    .map(|icon| icon.size.y)
                    .fold(0.0_f32, f32::max);
                let height = base_line_height.max(tallest_icon);
                let text_height = layout_line.max_ascent + layout_line.max_descent;
                let baseline = line_top + (height - text_height) * 0.5 + layout_line.max_ascent;
                for glyph in &layout_line.glyphs {
                    if is_bidi_control_cluster(line_text, glyph.start, glyph.end) {
                        continue;
                    }
                    let span_index = glyph
                        .metadata
                        .checked_sub(1)
                        .ok_or_else(|| "Text glyph is missing semantic metadata".to_string())?;
                    let span = markup
                        .spans
                        .get(span_index)
                        .ok_or_else(|| "Text glyph metadata is out of bounds".to_string())?;
                    if let Some(index) = span.object {
                        let icon = objects
                            .get(index)
                            .ok_or_else(|| "Text icon metadata is out of bounds".to_string())?;
                        layout.items.push(LayoutItem::Icon(PlacedIcon {
                            icon: icon.icon.clone(),
                            pos: vec2(glyph.x - min_x, line_top + (height - icon.size.y) * 0.5),
                            size: icon.size,
                            color: span.color,
                        }));
                    } else {
                        let physical = glyph.physical((0.0, 0.0), resolution);
                        layout.items.push(LayoutItem::Glyph(PlacedGlyph {
                            key: physical.cache_key,
                            pos: vec2(
                                physical.x as f32 / resolution - min_x,
                                physical.y as f32 / resolution + baseline,
                            ),
                            color: span.color,
                        }));
                    }
                }
                let size = vec2(layout_line.w, height);
                content_width = content_width.max(size.x);
                layout.lines.push(rich::RichTextLine {
                    offset_y: line_top,
                    size,
                });
                self.temp_line_items.push(start..layout.items.len());
                line_top += height;
            }
        }
        if seen.iter().any(|count| *count != 1) {
            return Err("Text icon placeholder did not produce exactly one glyph".into());
        }
        validate_finite(content_width, "Text layout width")?;
        validate_finite(line_top, "Text layout height")?;
        for (line, item_range) in layout.lines.iter().zip(&self.temp_line_items) {
            let offset = match text.h_align {
                HAlign::Left => 0.0,
                HAlign::Center => (content_width - line.size.x) * 0.5,
                HAlign::Right => content_width - line.size.x,
            };
            for item in &mut layout.items[item_range.clone()] {
                match item {
                    LayoutItem::Glyph(glyph) => glyph.pos.x += offset,
                    LayoutItem::Icon(icon) => icon.pos.x += offset,
                }
            }
        }
        let outline_radius = outline_radius(text.outline_width, resolution, pixelated);
        let outline_pad = outline_radius * 2.0;
        let size = vec2(content_width + outline_pad, line_top + outline_pad);
        validate_finite(size.x, "Text layout width")?;
        validate_finite(size.y, "Text layout height")?;
        layout.size = size;
        layout.resolution = resolution;
        layout.pixelated = pixelated;
        layout.outline_width = text.outline_width;
        layout.outline_radius = outline_radius;
        Ok(())
    }

    pub(crate) fn ensure_layout(&mut self, layout: &TextLayout) -> Result<(), String> {
        let mut resets = [false; TextAtlas::COUNT];
        loop {
            match self.ensure_sources(layout)? {
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

    pub(crate) fn resolve_layout(&self, layout: &TextLayout, quads: &mut Vec<QuadData>) {
        quads.clear();
        quads.reserve(layout.items.len());
        for item in &layout.items {
            match item {
                LayoutItem::Glyph(glyph) => self.resolve_glyph(layout, glyph, quads),
                LayoutItem::Icon(icon) => self.resolve_icon(icon, quads),
            }
        }
    }

    fn resolve_glyph(&self, layout: &TextLayout, glyph: &PlacedGlyph, quads: &mut Vec<QuadData>) {
        let normal_key = GlyphCacheKey {
            key: glyph.key,
            outline: 0,
        };
        let Some(info) = self.cache.get(&normal_key) else {
            return;
        };
        let Some(atlas) = info.atlas else { return };
        let atlas_size = self.atlas(atlas).texture.size();
        let atlas_glyph_size = info.size.as_vec2();
        let mut size = atlas_glyph_size / layout.resolution;
        if layout.pixelated {
            size = size.round();
        }
        let mut xy =
            glyph.pos + info.pos.as_vec2() / layout.resolution + Vec2::splat(layout.outline_radius);
        if layout.pixelated {
            xy = xy.round();
        }
        let outline = if layout.outline_width > 0 {
            let key = GlyphCacheKey {
                key: glyph.key,
                outline: layout.outline_width,
            };
            self.cache.get(&key).map(|info| {
                let texture_size = self.mask.texture.size();
                let outline_size = info.size.as_vec2();
                OutlineQuad {
                    xy: xy - Vec2::splat(layout.outline_radius),
                    size: size + Vec2::splat(layout.outline_radius * 2.0),
                    uvs1: info.atlas_pos / texture_size,
                    uvs2: (info.atlas_pos + outline_size) / texture_size,
                    source: TextSource::mask(layout.pixelated),
                }
            })
        } else {
            None
        };
        quads.push(QuadData {
            xy,
            size,
            uvs1: info.atlas_pos / atlas_size,
            uvs2: (info.atlas_pos + atlas_glyph_size) / atlas_size,
            source: atlas.source(layout.pixelated),
            color: glyph.color,
            pixelated: layout.pixelated,
            outline,
        });
    }

    fn resolve_icon(&self, icon: &PlacedIcon, quads: &mut Vec<QuadData>) {
        let key = IconCacheKey::from(&icon.icon);
        let Some(info) = self.icon_cache.get(&key) else {
            return;
        };
        let texture_size = self.atlas(info.atlas).texture.size();
        let source_size = icon.icon.source_size().as_vec2();
        let inner = info.outer_pos + Vec2::ONE;
        quads.push(QuadData {
            xy: icon.pos,
            size: icon.size,
            uvs1: inner / texture_size,
            uvs2: (inner + source_size) / texture_size,
            source: info.atlas.source(false),
            color: icon.color,
            pixelated: false,
            outline: None,
        });
    }

    fn atlas(&self, atlas: TextAtlas) -> &AtlasData {
        match atlas {
            TextAtlas::Mask => &self.mask,
            TextAtlas::RgbaLinear => &self.rgba_linear,
            TextAtlas::RgbaNearest => &self.rgba_nearest,
        }
    }

    fn atlas_mut(&mut self, atlas: TextAtlas) -> &mut AtlasData {
        match atlas {
            TextAtlas::Mask => &mut self.mask,
            TextAtlas::RgbaLinear => &mut self.rgba_linear,
            TextAtlas::RgbaNearest => &mut self.rgba_nearest,
        }
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

    fn ensure_sources(&mut self, layout: &TextLayout) -> Result<ProcessResult, String> {
        for item in &layout.items {
            match item {
                LayoutItem::Glyph(glyph) => {
                    if let Some(full) = self.ensure_glyph(glyph.key, layout.outline_width)? {
                        return Ok(ProcessResult::Full(full));
                    }
                }
                LayoutItem::Icon(icon) => {
                    if let Some(full) = self.ensure_icon(&icon.icon)? {
                        return Ok(ProcessResult::Full(full));
                    }
                }
            }
        }
        Ok(ProcessResult::Ready)
    }

    fn ensure_glyph(
        &mut self,
        key: CacheKey,
        outline_width: u16,
    ) -> Result<Option<TextAtlas>, String> {
        let normal_key = GlyphCacheKey { key, outline: 0 };
        let normal_cached = self.cache.contains_key(&normal_key);
        if outline_width > 0 {
            let outline_key = GlyphCacheKey {
                key,
                outline: outline_width,
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
                    &self.mask,
                )?;
                expanded_mask(&mut self.temp_outline_buff, &image.data, &outline)?;
                let Some(atlas_pos) = self.mask.store(outline.size, &self.temp_outline_buff)?
                else {
                    return Ok(Some(TextAtlas::Mask));
                };
                self.cache.insert(
                    outline_key,
                    GlyphInfo {
                        pos: outline.pos,
                        size: outline.cache_size,
                        atlas_pos,
                        atlas: Some(TextAtlas::Mask),
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
            SwashContent::Mask => TextAtlas::Mask,
            SwashContent::Color => TextAtlas::RgbaLinear,
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

fn validate_layout_metrics(
    text: &TextInfo,
    pixelated: bool,
    ppem: f32,
    res_ppem: f32,
    line_height_pem: f32,
) -> Result<(f32, f32, f32), String> {
    if text.strict_metrics {
        validate_positive(text.font_size, "Text size")?;
        validate_positive(text.resolution, "Text resolution")?;
        if let Some(height) = text.line_height {
            validate_positive(height, "Text line height")?;
        }
        if let Some(width) = text.wrap_width {
            validate_positive(width, "Text maximum width")?;
        }
    }
    validate_finite(text.font_size, "Text size")?;
    validate_finite(text.resolution, "Text resolution")?;
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
    let resolution = if pixelated {
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
        text.resolution * snapped_size / font_size
    } else {
        text.resolution
    };
    validate_positive(resolution, "Text effective resolution")?;

    let line_height = text.line_height.unwrap_or(font_size * line_height_pem);
    if text.line_height.is_some() {
        validate_finite(line_height, "Text effective line height")?;
    } else {
        validate_positive(line_height, "Text effective line height")?;
    }
    Ok((font_size, resolution, line_height))
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

fn is_bidi_formatting_control(character: char) -> bool {
    matches!(
        character,
        '\u{202A}'
            | '\u{202B}'
            | '\u{202C}'
            | '\u{202D}'
            | '\u{202E}'
            | '\u{2066}'
            | '\u{2067}'
            | '\u{2068}'
            | '\u{2069}'
    )
}

fn is_bidi_control_cluster(text: &str, start: usize, end: usize) -> bool {
    text.get(start..end).is_some_and(|cluster| {
        !cluster.is_empty() && cluster.chars().all(is_bidi_formatting_control)
    })
}

fn patch_shape(
    text: &str,
    shape: &mut ShapeLine,
    spans: &[markup::MarkupSpan],
    objects: &[PlacedIcon],
    seen: &mut [u8],
) -> Result<(), String> {
    for span in &mut shape.spans {
        for word in &mut span.words {
            for glyph in &mut word.glyphs {
                if is_bidi_control_cluster(text, glyph.start, glyph.end) {
                    glyph.x_advance = 0.0;
                    glyph.y_advance = 0.0;
                    glyph.ascent = 0.0;
                    glyph.descent = 0.0;
                    continue;
                }

                let span_index = glyph
                    .metadata
                    .checked_sub(1)
                    .ok_or_else(|| "Text glyph is missing semantic metadata".to_string())?;
                let semantic = spans
                    .get(span_index)
                    .ok_or_else(|| "Text glyph metadata is out of bounds".to_string())?;
                let Some(index) = semantic.object else {
                    continue;
                };
                let object = objects
                    .get(index)
                    .ok_or_else(|| "Text icon metadata is out of bounds".to_string())?;
                if text.get(glyph.start..glyph.end) != Some("\u{FFFC}") {
                    return Err("Text icon metadata does not refer to an object placeholder".into());
                }
                let count = seen
                    .get_mut(index)
                    .ok_or_else(|| "Text icon metadata is out of bounds".to_string())?;
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| "Text icon placeholder was repeated too often".to_string())?;
                if *count > 1 {
                    return Err("Text icon placeholder produced multiple glyphs".into());
                }
                glyph.x_advance = object.size.x / object.size.y;
                glyph.metrics_opt = Some(Metrics::new(object.size.y, object.size.y));
                glyph.y_advance = 0.0;
                glyph.ascent = 0.0;
                glyph.descent = 0.0;
            }
        }
    }
    Ok(())
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
    Mask,
    RgbaLinear,
    RgbaNearest,
}

impl TextAtlas {
    const COUNT: usize = 3;

    fn index(self) -> usize {
        match self {
            Self::Mask => 0,
            Self::RgbaLinear => 1,
            Self::RgbaNearest => 2,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Mask => "mask",
            Self::RgbaLinear => "linear RGBA",
            Self::RgbaNearest => "nearest RGBA",
        }
    }

    fn source(self, pixelated: bool) -> TextSource {
        match self {
            Self::Mask => TextSource::mask(pixelated),
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
}

impl TextSource {
    fn mask(pixelated: bool) -> Self {
        if pixelated {
            Self::MaskNearest
        } else {
            Self::MaskLinear
        }
    }

    pub(crate) fn selector(self) -> f32 {
        match self {
            Self::MaskLinear => 0.0,
            Self::MaskNearest => 1.0,
            Self::RgbaLinear => 2.0,
            Self::RgbaNearest => 3.0,
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

fn outline_radius(width: u16, resolution: f32, pixelated: bool) -> f32 {
    let radius = f32::from(width) / resolution;
    if pixelated { radius.round() } else { radius }
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
