use crate::m2d::Painter2D;
use crate::{Sprite, SpriteBuilder};
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use core::gfx::{self, BindGroup, RenderPipeline, Sampler, Texture, TextureFilter, TextureFormat};
use core::math::{uvec2, vec2, UVec2, Vec2};
use cosmic_text::fontdb::{Source, ID};
use cosmic_text::{
    Align, Attrs, Buffer, CacheKey, Family, FontSystem, LayoutGlyph, LayoutRun, Metrics,
    PhysicalGlyph, Shaping, Stretch, Style, SwashCache, SwashContent, Weight,
};
use etagere::{size2, Allocation, BucketedAtlasAllocator};
use hashbrown::HashSet;
use once_cell::sync::Lazy;
use rustc_hash::FxHasher;
use std::hash::BuildHasherDefault;
use std::sync::Arc;
use utils::fast_cache::FastCache;

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
    raw: ID,
    family: Arc<String>,
    weight: Weight,
    style: Style,
    stretch: Stretch,
    // TODO DropObserver, seems that cosmic-text doesn't have a way to remove fonts right now
}

#[derive(Debug)]
pub struct GlyphData {
    pub xy: Vec2,
    pub size: Vec2,
    pub uvs1: Vec2,
    pub uvs2: Vec2,
    pub typ: AtlasType,
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

impl HAlign {
    fn to_cosmic(&self) -> Align {
        match self {
            HAlign::Left => Align::Left,
            HAlign::Center => Align::Center,
            HAlign::Right => Align::Right,
        }
    }
}

pub struct TextInfo<'a> {
    pub pos: Vec2,
    pub font: Option<&'a Font>,
    pub text: &'a str,
    pub wrap_width: Option<f32>,
    pub font_size: f32,
    pub line_height: Option<f32>,
    pub scale: f32,
    pub h_align: HAlign,
}

struct ProcessData {
    key: CacheKey,
    pos: Vec2,
    line_w: f32,
    line_y: f32,
}

pub struct TextSystem {
    mask: AtlasData,
    color: AtlasData,
    sampler: Sampler,
    cache: FastCache<CacheKey, GlyphInfo>,
    font_system: FontSystem,
    swash: SwashCache,
    buffer: Buffer,
    font_ids: u64,
    default_font: Option<Font>,

    bind_group: Option<BindGroup>,

    // used to calculate rendering data
    process_data: Vec<ProcessData>,
    temp_data: Vec<GlyphData>,
}

impl TextSystem {
    pub fn new() -> Result<Self, String> {
        // common
        let max_texture_size = gfx::limits().max_texture_size_2d;
        let sampler = gfx::create_sampler()
            .with_min_filter(TextureFilter::Linear)
            .with_mag_filter(TextureFilter::Linear)
            .build()?;

        // mask atlas
        let allocator = BucketedAtlasAllocator::new(size2(
            DEFAULT_TEXTURE_SIZE as _,
            DEFAULT_TEXTURE_SIZE as _,
        ));
        let texture = gfx::create_texture()
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

        // mask atlas
        let allocator = BucketedAtlasAllocator::new(size2(
            DEFAULT_TEXTURE_SIZE as _,
            DEFAULT_TEXTURE_SIZE as _,
        ));
        let texture = gfx::create_texture()
            .with_empty_size(DEFAULT_TEXTURE_SIZE, DEFAULT_TEXTURE_SIZE)
            .with_write_flag(true)
            .build()?;

        let color = AtlasData {
            allocator,
            texture,
            max_texture_size,
            current_size: DEFAULT_TEXTURE_SIZE,
        };

        let cache = FastCache::default();

        let mut font_system = FontSystem::new();
        let swash = SwashCache::new();

        let buffer = Buffer::new(&mut font_system, Metrics::new(1.0, 1.0));

        let mut sys = Self {
            mask,
            color,
            sampler,
            cache,
            font_system,
            swash,
            buffer,
            font_ids: 0,
            default_font: None,

            bind_group: None,

            process_data: vec![],
            temp_data: vec![],
        };

        #[cfg(feature = "default-font")]
        {
            let font = sys.create_font(include_bytes!(
                "./resources/arcade-legacy/arcade-legacy.ttf"
            ))?;
            sys.default_font = Some(font);
        }

        Ok(sys)
    }

    pub fn bind_group(&mut self, pip: &RenderPipeline) -> &BindGroup {
        if self.bind_group.is_none() {
            log::info!("New text atlas bind_group created");
            let bg = gfx::create_bind_group()
                .with_sampler(0, &self.sampler)
                .with_texture(1, &self.mask.texture)
                .with_texture(2, &self.color.texture)
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

    pub fn create_font(&mut self, data: &'static [u8]) -> Result<Font, String> {
        let id = self.font_ids;
        self.font_ids += 1;
        let ids = self
            .font_system
            .db_mut()
            .load_font_source(Source::Binary(Arc::new(data)));
        let raw_id = ids
            .get(0)
            .ok_or_else(|| "Cannot create the font".to_string())?
            .clone();
        let face = self
            .font_system
            .db()
            .face(raw_id)
            .ok_or_else(|| "Invalid font type".to_string())?;
        Ok(Font {
            id: FontId(id),
            raw: raw_id,
            family: Arc::new(face.families[0].0.to_string()),
            weight: face.weight,
            style: face.style,
            stretch: face.stretch,
        })
    }

    pub fn prepare_text(&mut self, text: &TextInfo) -> Result<BlockInfo, String> {
        // clean the keys before we process a new text
        self.process_data.clear();

        // start processing the new text with the data provided by the user
        let font = text.font.or(self.default_font.as_ref());
        let attrs = match font {
            Some(f) => Attrs::new()
                .family(Family::Name(&f.family))
                .weight(f.weight)
                .style(f.style)
                .stretch(f.stretch),
            None => Attrs::new(),
        };

        let line_height = text.line_height.unwrap_or(text.font_size * 1.2);
        let metrics = Metrics::new(text.font_size, line_height);
        self.buffer.set_metrics(&mut self.font_system, metrics);
        self.buffer
            .set_size(&mut self.font_system, text.wrap_width, None);
        self.buffer
            .set_text(&mut self.font_system, text.text, attrs, Shaping::Advanced);

        self.buffer.shape_until_scroll(&mut self.font_system, false);

        match self.process(text.scale)? {
            PostAction::Restore => {
                self.restore();
                self.prepare_text(text)
            }
            PostAction::Clear => {
                self.clear()?;
                self.prepare_text(text)
            }
            PostAction::End { block_size, lines } => {
                // cleaning the temporal data shared with the user at the end
                self.temp_data.clear();

                let processed = self.process_data.iter().filter_map(|data| {
                    let Some(info) = self.cache.get(&data.key) else {
                        return None;
                    };

                    if matches!(info.typ, AtlasType::None) {
                        return None;
                    }
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

                    let pos = text.pos + data.pos + info.pos.as_vec2();
                    let xy = pos - offset + vec2(0.0, data.line_y);
                    let glyph_size = info.size.as_vec2();

                    Some(GlyphData {
                        xy,
                        size: glyph_size,
                        uvs1: info.atlas_pos / tex_size,
                        uvs2: (info.atlas_pos + glyph_size) / tex_size,
                        typ: info.typ,
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
        log::info!("Restoring TextAtlas glyphs.",);

        // TODO eventually add gfx::copy_texture_to_texture should be more efficient
        for (key, glyph) in self.cache.iter() {
            let atlas = match glyph.typ {
                AtlasType::Mask => &mut self.mask,
                AtlasType::Color => &mut self.color,
                AtlasType::None => continue,
            };

            let Some(image) = self.swash.get_image_uncached(&mut self.font_system, *key) else {
                continue;
            };

            let offset = uvec2(glyph.atlas_pos.x as _, glyph.atlas_pos.y as _);
            let size = uvec2(glyph.size.x as _, glyph.size.y as _);
            atlas.upload(size, offset, &image.data).unwrap();
        }

        self.bind_group = None;
    }

    fn process(&mut self, scale: f32) -> Result<PostAction, String> {
        let mut width: f32 = 0.0;
        let mut total_lines: usize = 0;

        for run in self.buffer.layout_runs() {
            width = run.line_w.max(width);
            total_lines += 1;

            for layout in run.glyphs {
                let glyph = layout.physical((0.0, 0.0), scale);

                // store to get rendering data later
                self.process_data.push(ProcessData {
                    key: glyph.cache_key,
                    pos: vec2(glyph.x as _, glyph.y as _),
                    line_w: run.line_w,
                    line_y: run.line_y,
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
            .map_err(|e| format!("Cannot clear Color text atlas: {}", e))?;
        self.mask
            .clear()
            .map_err(|e| format!("Cannot clear Mask text atlas: {}", e))?;

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

enum AtlasGrowError {
    MaxSizeReached,
    Gfx(String),
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum AtlasType {
    None,
    Mask,
    Color,
}

struct AtlasData {
    allocator: BucketedAtlasAllocator,
    texture: Texture,
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
        log::info!("Uploading new glyph to texture");
        gfx::write_texture(&self.texture)
            .from_data(&data)
            .with_offset(offset)
            .with_size(size)
            .build()
    }

    fn grow(&mut self) -> Result<bool, String> {
        let next_size = self.current_size * 2;
        if next_size > self.max_texture_size {
            log::info!("Max text atlas size reached.");
            return Ok(false);
        }

        log::info!(
            "Growing text atlas from {} to {}",
            self.current_size,
            next_size
        );
        self.allocator.grow(size2(next_size as _, next_size as _));

        self.texture = gfx::create_texture()
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
