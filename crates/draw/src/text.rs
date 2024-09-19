use crate::{Sprite, SpriteBuilder};
use core::gfx::{self, Sampler, Texture, TextureFilter, TextureFormat};
use core::math::{uvec2, vec2, UVec2, Vec2};
use cosmic_text::fontdb::{Source, ID};
use cosmic_text::{
    Attrs, Buffer, CacheKey, Family, FontSystem, LayoutGlyph, LayoutRun, Metrics, PhysicalGlyph,
    Shaping, Stretch, Style, SwashCache, SwashContent, Weight,
};
use etagere::{size2, Allocation, BucketedAtlasAllocator};
use std::sync::Arc;
use utils::fast_cache::FastCache;

const DEFAULT_TEXTURE_SIZE: u32 = 64;
const ATLAS_OFFSET: u32 = 1;

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
    // TODO DropObserver, however seems that cosmic-text doesn't have a way to remove fonts right now
}

pub struct TextInfo<'a> {
    pub font: Option<&'a Font>,
    pub text: &'a str,
    pub wrap_width: Option<f32>,
    pub font_size: f32,
    pub line_height: Option<f32>,
    pub scale: f32,
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

    pub fn mask_texture(&self) -> Sprite {
        SpriteBuilder::new()
            .from_texture(&self.mask.texture)
            .with_sampler(&self.sampler)
            .build()
            .unwrap()
    }

    pub fn color_texture(&self) -> Sprite {
        SpriteBuilder::new()
            .from_texture(&self.color.texture)
            .with_sampler(&self.sampler)
            .build()
            .unwrap()
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

    // TODO FRAME STARTS so we know what glyphs needs to be renders this frame?
    // in case we need to re-start because we rebuild the textures we can show all
    // glyphs instead of only some (think of rebuilding tex in the middle of the buffer)

    pub fn prepare_text(&mut self, text: &TextInfo) {
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

        self.process(text.scale);
    }

    fn process(&mut self, scale: f32) {
        for run in self.buffer.layout_runs() {
            for layout in run.glyphs {
                let glyph = layout.physical((0.0, 0.0), scale);
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
                };

                let atlas_pos = match atlas.store(uvec2(width, height), &image.data).unwrap() {
                    Some(pos) => pos,
                    None => {
                        let grow = atlas.grow().unwrap();
                        if !grow {
                            log::info!("Max text atlas texture reached, reseting...");
                        }

                        // we clear the atlas no matter if it grows or not, so next frame we get all the glyps again
                        // TODO we may want a way to redraw all glyph on this frame to avoid a one frame visual glitch
                        self.clear();
                        return;
                    }
                };

                let info = GlyphInfo {
                    pos: Pos::new(image.placement.left as _, -image.placement.top as _),
                    size: Pos::new(image.placement.width as _, image.placement.height as _),
                    atlas_pos,
                };
                self.cache.insert(glyph.cache_key, info);
            }
        }
    }

    fn clear(&mut self) {
        self.cache.clear();
        self.color.clear();
        self.mask.clear();
    }
}

enum AtlasGrowError {
    MaxSizeReached,
    Gfx(String),
}

enum AtlasType {
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
            (size.x + ATLAS_OFFSET) as _,
            (size.y + ATLAS_OFFSET) as _,
        ));

        let alloc = match alloc {
            Some(alloc) => alloc,
            None => {
                return Ok(None);
            }
        };

        let offset = uvec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _);

        gfx::write_texture(&self.texture)
            .from_data(&data)
            .with_offset(offset)
            .with_size(size)
            .build()?;

        Ok(Some(vec2(
            alloc.rectangle.min.x as _,
            alloc.rectangle.min.y as _,
        )))
    }

    fn grow(&mut self) -> Result<bool, String> {
        let next_size = self.current_size * 2;
        if next_size > self.max_texture_size {
            return Ok(false);
        }

        self.allocator.clear();
        self.allocator.grow(size2(next_size as _, next_size as _));

        self.texture = gfx::create_texture()
            .with_empty_size(next_size, next_size)
            .with_format(self.texture.format())
            .with_write_flag(true)
            .build()?;

        self.current_size = next_size;

        Ok(true)
    }

    fn clear(&mut self) {
        self.allocator.clear();
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
}
