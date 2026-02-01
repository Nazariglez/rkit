use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use corelib::gfx::{
    self, BindGroup, Color, RenderPipeline, Sampler, Texture, TextureFilter, TextureFormat,
};
use corelib::math::{UVec2, Vec2, uvec2, vec2};
use cosmic_text::fontdb::Source;
use cosmic_text::{
    Attrs, Buffer, CacheKey, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, Stretch,
    Style, SwashCache, SwashContent, Weight,
};
use etagere::{BucketedAtlasAllocator, size2};
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;
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
    line_height_per_em: f32,
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

#[derive(Debug)]
pub struct GlyphData {
    pub xy: Vec2,
    pub size: Vec2,
    pub uvs1: Vec2,
    pub uvs2: Vec2,
    pub(crate) typ: AtlasType,
    pub(crate) pixelated: bool,
    pub color: Option<Color>,
}

pub struct BlockInfo<'a> {
    pub size: Vec2,
    pub lines: usize,
    pub data: &'a [GlyphData],
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Default)]
pub enum HAlign {
    #[default]
    Left,
    Center,
    Right,
}

pub struct TextInfo<'a> {
    pub pos: Vec2,
    pub font: Option<&'a Font>,
    pub text: &'a str,
    pub wrap_width: Option<f32>,
    pub font_size: f32,
    pub line_height: Option<f32>,
    pub resolution: f32,
    pub h_align: HAlign,
    pub color_tags: bool,
    pub default_color: Color,
}

pub fn text_metrics(text: &str) -> TextMetricsBuilder<'_> {
    TextMetricsBuilder {
        info: TextInfo {
            pos: Default::default(),
            font: None,
            text,
            wrap_width: None,
            font_size: 14.0,
            line_height: None,
            resolution: 1.0,
            h_align: Default::default(),
            color_tags: false,
            default_color: Color::WHITE,
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

    pub fn measure(self) -> TextMetrics {
        let BlockInfo { size, lines, .. } = get_mut_text_system()
            .prepare_text(&self.info, true)
            .unwrap();
        TextMetrics { size, lines }
    }
}

struct ProcessData {
    key: CacheKey,
    pos: Vec2,
    line_w: f32,
    line_y: f32,
    color_opt: Option<CosmicColor>,
}

pub struct TextSystem {
    pub(crate) mask: AtlasData,
    pub(crate) color: AtlasData,
    linear_sampler: Sampler,
    nearest_sampler: Sampler,
    cache: FxHashMap<CacheKey, GlyphInfo>,
    font_system: FontSystem,
    swash: SwashCache,
    buffer: Buffer,
    font_ids: u64,
    default_font: Option<Font>,

    bind_group: Option<BindGroup>,

    // used to calculate rendering data
    process_data: Vec<ProcessData>,
    temp_data: Vec<GlyphData>,

    // reusable string to avoid allocations
    temp_hack_string: String,

    // storage for parsed color tag spans so we avoid allocations
    span_ranges: Vec<(Range<usize>, Option<Color>)>,
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

        // mask atlas
        let allocator = BucketedAtlasAllocator::new(size2(
            DEFAULT_TEXTURE_SIZE as _,
            DEFAULT_TEXTURE_SIZE as _,
        ));
        let texture = gfx::create_texture()
            .with_label("TextSystem Texture Mask")
            .with_empty_size(DEFAULT_TEXTURE_SIZE, DEFAULT_TEXTURE_SIZE)
            .with_format(TextureFormat::R8UNorm)
            .with_write_flag(true)
            .build()?;

        let mask = AtlasData {
            allocator,
            texture,
            max_texture_size,
            current_size: DEFAULT_TEXTURE_SIZE,
        };

        // color atlas
        let allocator = BucketedAtlasAllocator::new(size2(
            DEFAULT_TEXTURE_SIZE as _,
            DEFAULT_TEXTURE_SIZE as _,
        ));
        let texture = gfx::create_texture()
            .with_label("TextSystem Texture Color")
            .with_empty_size(DEFAULT_TEXTURE_SIZE, DEFAULT_TEXTURE_SIZE)
            .with_write_flag(true)
            .build()?;

        let color = AtlasData {
            allocator,
            texture,
            max_texture_size,
            current_size: DEFAULT_TEXTURE_SIZE,
        };

        let cache = FxHashMap::default();

        let mut font_system = FontSystem::new();
        let swash = SwashCache::new();

        let buffer = Buffer::new(&mut font_system, Metrics::new(1.0, 1.0));

        #[allow(unused_mut)]
        let mut sys = Self {
            mask,
            color,
            linear_sampler,
            nearest_sampler,
            cache,
            font_system,
            swash,
            buffer,
            font_ids: 0,
            default_font: None,

            bind_group: None,

            process_data: vec![],
            temp_data: vec![],
            temp_hack_string: String::new(),
            span_ranges: vec![],
        };

        #[cfg(feature = "default-font")]
        {
            let font = sys.create_font(
                include_bytes!("./resources/arcade-legacy/arcade-legacy.ttf"),
                true,
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
                .with_texture(3, &self.color.texture)
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

    pub fn create_font(&mut self, data: &[u8], nearest: bool) -> Result<Font, String> {
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
        let desc = -metrics.descent;
        let lead = metrics.leading;
        let line_height_per_em = (asc + desc + lead) / upm;
        let px_per_em = upm / metrics.cap_height;

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
            line_height_per_em,
            family: Arc::new(face.families[0].0.to_string()),
            weight: face.weight,
            style: face.style,
            stretch: face.stretch,
        })
    }

    pub fn prepare_text(
        &mut self,
        text: &TextInfo,
        only_measure: bool,
    ) -> Result<BlockInfo<'_>, String> {
        // clean the keys before we process a new text
        self.process_data.clear();

        // start processing the new text with the data provided by the user
        let font = text.font.or(self.default_font.as_ref());
        let (pixelated, ppem, res_ppem, lh_pem) = font
            .map(|f| {
                (
                    f.is_pixelated(),
                    f.px_per_em,
                    f.res_ppem,
                    f.line_height_per_em,
                )
            })
            .unwrap_or((false, 1.0, 1.0, 1.0));
        let attrs = match font {
            Some(f) => Attrs::new()
                .family(Family::Name(&f.family))
                .weight(f.weight)
                .style(f.style)
                .stretch(f.stretch),
            None => Attrs::new(),
        };

        let font_size = text.font_size * ppem;
        let resolution = if pixelated {
            let next_fs = closest_multiple_of(font_size as _, res_ppem as _) as f32;
            let scale = next_fs / font_size;
            text.resolution * scale
        } else {
            // TODO: for regular fonts we may want to pass the dpi so we
            // scale it related to the screen space to look better?
            text.resolution
        };

        let line_height = text.line_height.unwrap_or(font_size * lh_pem * 1.4);
        let metrics = Metrics::new(font_size, line_height);
        self.buffer.set_metrics(&mut self.font_system, metrics);
        self.buffer
            .set_size(&mut self.font_system, text.wrap_width, None);

        if text.color_tags {
            self.span_ranges.clear();
            self.span_ranges
                .extend(parse_color_tag_ranges(text.text, text.default_color));

            // TODO: remove it when cosmic-text adds a fix for https://github.com/pop-os/cosmic-text/issues/251
            if text.wrap_width.is_some() {
                if let Some(hinted) = try_add_zwsp_hints_ranges(
                    &mut self.temp_hack_string,
                    text.text,
                    &self.span_ranges,
                ) {
                    let cosmic_spans = hinted.iter().map(|(range, color)| {
                        (
                            &self.temp_hack_string[range.clone()],
                            color_attrs(attrs, *color),
                        )
                    });
                    self.buffer.set_rich_text(
                        &mut self.font_system,
                        cosmic_spans,
                        attrs,
                        Shaping::Advanced,
                    );
                } else {
                    let cosmic_spans = self.span_ranges.iter().map(|(range, color)| {
                        (&text.text[range.clone()], color_attrs(attrs, *color))
                    });
                    self.buffer.set_rich_text(
                        &mut self.font_system,
                        cosmic_spans,
                        attrs,
                        Shaping::Advanced,
                    );
                }
            } else {
                let cosmic_spans = self
                    .span_ranges
                    .iter()
                    .map(|(range, color)| (&text.text[range.clone()], color_attrs(attrs, *color)));
                self.buffer.set_rich_text(
                    &mut self.font_system,
                    cosmic_spans,
                    attrs,
                    Shaping::Advanced,
                );
            }
        } else if text.wrap_width.is_some() {
            // TODO: remove it when comisc-text adds a fix for https://github.com/pop-os/cosmic-text/issues/251
            let fix_wrap_text = add_zwsp_hints(&mut self.temp_hack_string, text.text);
            self.buffer.set_text(
                &mut self.font_system,
                &fix_wrap_text,
                attrs,
                Shaping::Advanced,
            );
        } else {
            self.buffer
                .set_text(&mut self.font_system, text.text, attrs, Shaping::Advanced);
        }

        self.buffer.shape_until_scroll(&mut self.font_system, false);

        // do not mess with textures when we only want the size of the block
        if only_measure {
            let (size, lines) = self.measure()?;
            return Ok(BlockInfo {
                size,
                lines,
                data: &[],
            });
        }

        match self.process(resolution)? {
            PostAction::Restore => {
                self.restore();
                self.prepare_text(text, false)
            }
            PostAction::Clear => {
                self.clear()?;
                self.prepare_text(text, false)
            }
            PostAction::End { block_size, lines } => {
                // cleaning the temporal data shared with the user at the end
                self.temp_data.clear();

                let processed = self.process_data.iter().filter_map(|data| {
                    let info = self.cache.get(&data.key)?;

                    let tex_size = match info.typ {
                        AtlasType::None => return None,
                        AtlasType::Mask => self.mask.texture.size(),
                        AtlasType::Color => self.color.texture.size(),
                    };

                    let offset = {
                        let x = match text.h_align {
                            HAlign::Left => 0.0,
                            HAlign::Center => 0.5,
                            HAlign::Right => 1.0,
                        };

                        let ww = (block_size.x - data.line_w) * -x;
                        vec2(ww, 0.0)
                    };

                    let atlas_size = info.size.as_vec2();
                    let screen_size = atlas_size / resolution;
                    let pos = text.pos + data.pos + (info.pos.as_vec2() / resolution);
                    let xy = pos - offset + vec2(0.0, data.line_y);

                    let glyph_color = data.color_opt.map(|c| {
                        let (r, g, b, a) = c.as_rgba_tuple();
                        Color::rgba_u8(r, g, b, a)
                    });

                    Some(GlyphData {
                        xy,
                        size: screen_size,
                        uvs1: info.atlas_pos / tex_size,
                        uvs2: (info.atlas_pos + atlas_size) / tex_size,
                        typ: info.typ,
                        pixelated,
                        color: glyph_color,
                    })
                });
                self.temp_data.extend(processed);

                Ok(BlockInfo {
                    size: block_size,
                    lines,
                    data: &self.temp_data,
                })
            }
        }
    }

    fn restore(&mut self) {
        log::debug!("Restoring TextAtlas glyphs.",);

        // TODO: eventually add gfx::copy_texture_to_texture should be more efficient
        for (key, glyph) in self.cache.iter() {
            let atlas = match glyph.typ {
                AtlasType::Mask => &mut self.mask,
                AtlasType::Color => &mut self.color,
                AtlasType::None => continue,
            };

            let Some(image) = self.swash.get_image_uncached(&mut self.font_system, *key) else {
                continue;
            };

            let offset = glyph.atlas_pos.as_uvec2();
            let size = uvec2(glyph.size.x as _, glyph.size.y as _);
            atlas.upload(size, offset, &image.data).unwrap();
        }

        self.bind_group = None;
    }

    fn measure(&mut self) -> Result<(Vec2, usize), String> {
        let mut width: f32 = 0.0;
        let mut total_lines: usize = 0;

        for run in self.buffer.layout_runs() {
            width = run.line_w.max(width);
            total_lines += 1;
        }

        let size = vec2(
            width,
            total_lines as f32 * (self.buffer.metrics().line_height),
        );
        Ok((size, total_lines))
    }

    fn process(&mut self, resolution: f32) -> Result<PostAction, String> {
        let mut width: f32 = 0.0;
        let mut total_lines: usize = 0;

        for run in self.buffer.layout_runs() {
            width = run.line_w.max(width);
            total_lines += 1;

            for layout in run.glyphs {
                let glyph = layout.physical((0.0, 0.0), resolution);

                // store to get rendering data later
                self.process_data.push(ProcessData {
                    key: glyph.cache_key,
                    pos: vec2(glyph.x as _, glyph.y as _) / resolution,
                    line_w: run.line_w,
                    line_y: run.line_y,
                    color_opt: layout.color_opt,
                });

                // if it's already in the main cache just skip it
                if self.cache.contains_key(&glyph.cache_key) {
                    continue;
                }

                let Some(image) = self
                    .swash
                    .get_image_uncached(&mut self.font_system, glyph.cache_key)
                else {
                    continue;
                };

                let width = image.placement.width;
                let height = image.placement.height;
                if width == 0 || height == 0 {
                    // if there is nothing to rasterize, then cache it to avoid getting the image but mark it as skipable
                    self.cache.insert(
                        glyph.cache_key,
                        GlyphInfo {
                            pos: Pos::new(0, 0),
                            size: Pos::new(0, 0),
                            atlas_pos: Default::default(),
                            typ: AtlasType::None,
                        },
                    );
                    continue;
                }

                let typ = match image.content {
                    SwashContent::Mask => AtlasType::Mask,
                    SwashContent::Color => AtlasType::Color,
                    SwashContent::SubpixelMask => continue, // not supported by cosmic-text yet
                };

                let atlas = match typ {
                    AtlasType::Mask => &mut self.mask,
                    AtlasType::Color => &mut self.color,
                    AtlasType::None => unreachable!("This should never happen"),
                };

                let atlas_pos = match atlas.store(uvec2(width, height), &image.data).unwrap() {
                    Some(pos) => pos,
                    None => {
                        let grow = atlas.grow()?;
                        if grow {
                            return Ok(PostAction::Restore);
                        } else {
                            return Ok(PostAction::Clear);
                        }
                    }
                };

                let info = GlyphInfo {
                    pos: Pos::new(image.placement.left as _, -image.placement.top as _),
                    size: Pos::new(image.placement.width as _, image.placement.height as _),
                    atlas_pos,
                    typ,
                };
                self.cache.insert(glyph.cache_key, info);
            }
        }

        let size = vec2(
            width,
            total_lines as f32 * self.buffer.metrics().line_height,
        );

        Ok(PostAction::End {
            block_size: size,
            lines: total_lines,
        })
    }

    fn clear(&mut self) -> Result<(), String> {
        self.color
            .clear()
            .map_err(|e| format!("Cannot clear Color text atlas: {e}"))?;
        self.mask
            .clear()
            .map_err(|e| format!("Cannot clear Mask text atlas: {e}"))?;

        self.cache.clear();
        self.temp_data.clear();

        Ok(())
    }
}

enum PostAction {
    End { block_size: Vec2, lines: usize },
    Restore,
    Clear,
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum AtlasType {
    None,
    Mask,
    Color,
}

pub(crate) struct AtlasData {
    pub(crate) texture: Texture,
    allocator: BucketedAtlasAllocator,
    max_texture_size: u32,
    current_size: u32,
}

impl AtlasData {
    fn store(&mut self, size: UVec2, data: &[u8]) -> Result<Option<Vec2>, String> {
        let alloc = self.allocator.allocate(size2(
            (size.x + ATLAS_PIXEL_OFFSET) as _,
            (size.y + ATLAS_PIXEL_OFFSET) as _,
        ));

        let alloc = match alloc {
            Some(alloc) => alloc,
            None => {
                return Ok(None);
            }
        };

        let offset = uvec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _);
        self.upload(size, offset, data)?;

        Ok(Some(vec2(
            alloc.rectangle.min.x as _,
            alloc.rectangle.min.y as _,
        )))
    }

    fn upload(&self, size: UVec2, offset: UVec2, data: &[u8]) -> Result<(), String> {
        log::trace!("Uploading new glyph to texture");
        gfx::write_texture(&self.texture)
            .from_data(data)
            .with_offset(offset)
            .with_size(size)
            .build()
    }

    fn grow(&mut self) -> Result<bool, String> {
        let next_size = self.current_size * 2;
        if next_size > self.max_texture_size {
            log::debug!("Max text atlas size reached.");
            return Ok(false);
        }

        log::debug!(
            "Growing text atlas from {} to {}",
            self.current_size,
            next_size
        );
        self.allocator.grow(size2(next_size as _, next_size as _));

        self.texture = gfx::create_texture()
            .with_label("TextSystem Texture")
            .with_empty_size(next_size, next_size)
            .with_format(self.texture.format())
            .with_write_flag(true)
            .build()?;

        self.current_size = next_size;

        Ok(true)
    }

    fn clear(&mut self) -> Result<(), String> {
        let channels = self.texture.format().channels();
        let len = self.texture.size().element_product() as usize * channels as usize;
        let empty = vec![0; len];

        gfx::write_texture(&self.texture)
            .from_data(&empty)
            .build()?;

        self.allocator.clear();

        Ok(())
    }
}

/// Comisc text have an issue when a wrap ends up on a whitespace
/// https://github.com/pop-os/cosmic-text/issues/251
/// this is a hackish way of avoid this for now
/// it's kind of expensive because we need to process the string
/// each time we renderer it, although I am not allocating a new
/// string each time but reusing it.
fn add_zwsp_hints<'a>(temp_hack_string: &mut String, s: &'a str) -> Cow<'a, str> {
    if !s.as_bytes().contains(&b' ') {
        return Cow::Borrowed(s);
    }

    let mut need = false;
    for w in s.as_bytes().windows(2) {
        if w[0] == b' ' {
            need = true;
            break;
        }
    }
    if !need {
        return Cow::Borrowed(s);
    }

    temp_hack_string.clear();
    for ch in s.chars() {
        temp_hack_string.push(ch);
        if ch == ' ' {
            temp_hack_string.push('\u{200B}');
        }
    }
    Cow::Owned(std::mem::take(temp_hack_string))
}

type ColorStack = SmallVec<Color, 4>;
type SpanRanges = SmallVec<(Range<usize>, Option<Color>), 8>;

#[inline]
fn color_attrs(attrs: Attrs, color: Option<Color>) -> Attrs {
    match color.map(|c| c.to_rgba_u8()) {
        Some([r, g, b, a]) => attrs.color(CosmicColor::rgba(r, g, b, a)),
        None => attrs,
    }
}

// TODO: remove it when cosmic-text adds a fix for https://github.com/pop-os/cosmic-text/issues/251
fn try_add_zwsp_hints_ranges(
    temp: &mut String,
    text: &str,
    span_ranges: &[(Range<usize>, Option<Color>)],
) -> Option<SpanRanges> {
    let mut prev_space = false;
    let mut needs_hint = false;
    for (range, _) in span_ranges {
        let span_text = &text[range.clone()];
        if span_text.is_empty() {
            continue;
        }
        let bytes = span_text.as_bytes();
        if prev_space && bytes[0] != b' ' {
            needs_hint = true;
            break;
        }
        for w in bytes.windows(2) {
            if w[0] == b' ' {
                needs_hint = true;
                break;
            }
        }
        if needs_hint {
            break;
        }
        prev_space = *bytes.last().unwrap() == b' ';
    }

    if !needs_hint {
        return None;
    }

    temp.clear();
    let mut ranges = SpanRanges::new();
    for (range, color) in span_ranges {
        let span_text = &text[range.clone()];
        let start = temp.len();
        for ch in span_text.chars() {
            temp.push(ch);
            if ch == ' ' {
                temp.push('\u{200B}');
            }
        }
        ranges.push((start..temp.len(), *color));
    }
    Some(ranges)
}

const COLOR_TAG_OPEN: &str = "[color:#";
const COLOR_TAG_CLOSE: &str = "[/color]";

#[inline]
fn try_parse_open_tag(s: &str) -> Option<(Color, usize)> {
    let close_bracket = s.find(']')?;
    let hex_str = &s[..close_bracket];
    let color = parse_hex_color(hex_str);
    Some((color, close_bracket + 1))
}

#[inline]
fn parse_hex_color(hex: &str) -> Color {
    let bytes = hex.as_bytes();
    let mut value: u32 = 0;

    for &b in bytes.iter().take(8) {
        let nibble = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => 0,
        };
        value = (value << 4) | nibble as u32;
    }

    if bytes.len() <= 6 {
        value = (value << 8) | 0xFF;
    }

    Color::hex(value)
}

fn parse_color_tag_ranges(input: &str, default_color: Color) -> SpanRanges {
    let mut ranges = SpanRanges::new();
    let mut color_stack: ColorStack = SmallVec::new();
    color_stack.push(default_color);
    let mut cursor = 0;
    let mut span_start = 0;

    while cursor < input.len() {
        let remaining = &input[cursor..];
        let open_pos = remaining.find(COLOR_TAG_OPEN);
        let close_pos = remaining.find(COLOR_TAG_CLOSE);

        match (open_pos, close_pos) {
            (Some(op), close) if close.is_none() || op < close.unwrap() => {
                if cursor + op > span_start {
                    ranges.push((span_start..cursor + op, Some(*color_stack.last().unwrap())));
                }
                let after_prefix = &remaining[op + COLOR_TAG_OPEN.len()..];
                if let Some((color, consumed)) = try_parse_open_tag(after_prefix) {
                    color_stack.push(color);
                    cursor += op + COLOR_TAG_OPEN.len() + consumed;
                    span_start = cursor;
                } else {
                    cursor += op + 1;
                }
            }
            (open, Some(cp)) if open.is_none() || cp < open.unwrap() => {
                if cursor + cp > span_start {
                    ranges.push((span_start..cursor + cp, Some(*color_stack.last().unwrap())));
                }
                if color_stack.len() > 1 {
                    color_stack.pop();
                }
                cursor += cp + COLOR_TAG_CLOSE.len();
                span_start = cursor;
            }
            (None, None) => {
                if span_start < input.len() {
                    ranges.push((span_start..input.len(), Some(*color_stack.last().unwrap())));
                }
                break;
            }
            _ => break,
        }
    }

    ranges
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

#[derive(Debug)]
struct GlyphInfo {
    pos: Pos<i16>,
    size: Pos<u16>,
    atlas_pos: Vec2,
    typ: AtlasType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_tags() {
        let ranges = parse_color_tag_ranges("Hello world", Color::WHITE);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].0, 0..11);
        assert_eq!(ranges[0].1, Some(Color::WHITE));
    }

    #[test]
    fn test_parse_single_color_tag() {
        let input = "Hello [color:#FF0000]red[/color] world";
        let ranges = parse_color_tag_ranges(input, Color::WHITE);
        assert_eq!(ranges.len(), 3);
        assert_eq!(&input[ranges[0].0.clone()], "Hello ");
        assert_eq!(ranges[0].1, Some(Color::WHITE));
        assert_eq!(&input[ranges[1].0.clone()], "red");
        assert_eq!(ranges[1].1, Some(Color::hex(0xFF0000FF)));
        assert_eq!(&input[ranges[2].0.clone()], " world");
        assert_eq!(ranges[2].1, Some(Color::WHITE));
    }

    #[test]
    fn test_parse_nested_color_tags() {
        let input = "[color:#FF0000]red [color:#00FF00]green[/color] back[/color]";
        let ranges = parse_color_tag_ranges(input, Color::WHITE);
        assert_eq!(ranges.len(), 3);
        assert_eq!(&input[ranges[0].0.clone()], "red ");
        assert_eq!(ranges[0].1, Some(Color::hex(0xFF0000FF)));
        assert_eq!(&input[ranges[1].0.clone()], "green");
        assert_eq!(ranges[1].1, Some(Color::hex(0x00FF00FF)));
        assert_eq!(&input[ranges[2].0.clone()], " back");
        assert_eq!(ranges[2].1, Some(Color::hex(0xFF0000FF)));
    }

    #[test]
    fn test_parse_with_alpha() {
        let input = "[color:#FF000080]semi-transparent[/color]";
        let ranges = parse_color_tag_ranges(input, Color::WHITE);
        assert_eq!(ranges.len(), 1);
        assert_eq!(&input[ranges[0].0.clone()], "semi-transparent");
        assert_eq!(ranges[0].1, Some(Color::hex(0xFF000080)));
    }

    #[test]
    fn test_parse_default_color() {
        let default = Color::hex(0x123456FF);
        let ranges = parse_color_tag_ranges("text", default);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].1, Some(default));
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("FF0000"), Color::hex(0xFF0000FF));
        assert_eq!(parse_hex_color("00FF00"), Color::hex(0x00FF00FF));
        assert_eq!(parse_hex_color("0000FF"), Color::hex(0x0000FFFF));
        assert_eq!(parse_hex_color("FF000080"), Color::hex(0xFF000080));
    }
}
