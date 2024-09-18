use core::gfx::{self, Sampler, Texture, TextureFilter, TextureFormat};
use core::math::{uvec2, vec2, UVec2, Vec2};
use cosmic_text::fontdb::{Source, ID};
use cosmic_text::{
    Buffer, CacheKey, FontSystem, LayoutGlyph, LayoutRun, PhysicalGlyph, Stretch, Style,
    SwashCache, SwashContent, Weight,
};
use etagere::{size2, BucketedAtlasAllocator};
use std::sync::Arc;
use utils::drop_signal::DropObserver;
use utils::fast_cache::FastCache;

const DEFAULT_TEXTURE_SIZE: u32 = 256;
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
    drop_observer: DropObserver,
}

pub struct TextSystem {
    mask: AtlasData,
    color: AtlasData,
    sampler: Sampler,
    cache: FastCache<CacheKey, GlyphInfo>,
    font_system: FontSystem,
    swash: SwashCache,
    max_texture_size: u32,
    font_ids: u64,
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
            .build()?;

        let mask = AtlasData {
            allocator,
            texture,
            current_size: DEFAULT_TEXTURE_SIZE,
        };

        // mask atlas
        let allocator = BucketedAtlasAllocator::new(size2(
            DEFAULT_TEXTURE_SIZE as _,
            DEFAULT_TEXTURE_SIZE as _,
        ));
        let texture = gfx::create_texture()
            .with_empty_size(DEFAULT_TEXTURE_SIZE, DEFAULT_TEXTURE_SIZE)
            .build()?;

        let color = AtlasData {
            allocator,
            texture,
            current_size: DEFAULT_TEXTURE_SIZE,
        };

        let cache = FastCache::default();

        let font_system = FontSystem::new();
        let swash = SwashCache::new();

        Ok(Self {
            mask,
            color,
            sampler,
            cache,
            font_system,
            swash,
            max_texture_size,
            font_ids: 0,
        })
    }

    fn create_font(&mut self, data: &'static [u8]) -> Result<Font, String> {
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
            drop_observer: Default::default(),
        })
    }

    // TODO FRAME STARTS so we know what glyphs needs to be renders this frame?
    // in case we need to re-start because we rebuild the textures we can show all
    // glyphs instead of only some (think of rebuilding tex in the middle of the buffer)

    fn prepare(&mut self, glyph: PhysicalGlyph) {
        if self.cache.contains_key(&glyph.cache_key) {
            return;
        }

        let Some(image) = self
            .swash
            .get_image_uncached(&mut self.font_system, glyph.cache_key)
        else {
            return;
        };

        let width = image.placement.width;
        let height = image.placement.height;
        if width == 0 || height == 0 {
            return;
        }

        let typ = match image.content {
            SwashContent::Mask => AtlasType::Mask,
            SwashContent::Color => AtlasType::Color,
            SwashContent::SubpixelMask => return, // not supported by cosmic-text yet
        };

        let atlas = match typ {
            AtlasType::Mask => &mut self.mask,
            AtlasType::Color => &mut self.color,
        };

        let atlas_pos = atlas.store(uvec2(width, height), &image.data).unwrap();
        let info = GlyphInfo {
            pos: Pos::new(image.placement.left as _, -image.placement.top as _),
            size: Pos::new(image.placement.width as _, image.placement.height as _),
            atlas_pos,
        };
        self.cache.insert(glyph.cache_key, info);
    }
}

enum AtlasType {
    Mask,
    Color,
}

struct AtlasData {
    allocator: BucketedAtlasAllocator,
    texture: Texture,
    current_size: u32,
}

impl AtlasData {
    fn store(&mut self, size: UVec2, data: &[u8]) -> Result<Vec2, String> {
        // TODO if alloc fails, we need to grow
        let alloc = self
            .allocator
            .allocate(size2(
                (size.x + ATLAS_OFFSET) as _,
                (size.y + ATLAS_OFFSET) as _,
            ))
            .ok_or_else(|| "Not enough space on atlas".to_string())?;

        let offset = uvec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _);

        gfx::write_texture(&self.texture)
            .from_data(&data)
            .with_offset(offset)
            .with_size(size)
            .build()?;

        Ok(vec2(alloc.rectangle.min.x as _, alloc.rectangle.min.y as _))
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
