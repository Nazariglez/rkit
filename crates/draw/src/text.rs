use core::gfx::{self, Sampler, Texture, TextureFilter, TextureFormat};
use core::math::{vec2, Vec2};
use cosmic_text::{
    Buffer, CacheKey, FontSystem, LayoutGlyph, LayoutRun, PhysicalGlyph, SwashCache, SwashContent,
};
use etagere::{size2, BucketedAtlasAllocator};
use utils::fast_cache::FastCache;

const DEFAULT_TEXTURE_SIZE: u32 = 256;
const ATLAS_OFFSET: u32 = 1;

pub struct TextSystem {
    mask: AtlasData,
    color: AtlasData,
    sampler: Sampler,
    cache: FastCache<CacheKey, GlyphInfo>,
    font_system: FontSystem,
    swash: SwashCache,
    max_texture_size: u32,
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

        let res = atlas.store(width, height, &image.data);
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
    fn store(&mut self, width: u32, height: u32, data: &[u8]) -> Result<Vec2, String> {
        // TODO if alloc fails, we need to grow
        let alloc = self
            .allocator
            .allocate(size2(
                (width + ATLAS_OFFSET) as _,
                (height + ATLAS_OFFSET) as _,
            ))
            .unwrap();

        // TODO upload texture here

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
