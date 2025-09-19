use crate::{SpriteBuilder, create_sprite};

use super::sprite::Sprite;
use corelib::{
    gfx::{
        RenderTexture, RenderTextureBuilder, Sampler, TextureFilter, TextureFormat, TextureWrap,
        create_render_texture,
    },
    math::UVec2,
};
use std::num::NonZeroUsize;
use utils::fast_cache::FastCache;

pub struct RenderSpriteCache {
    pub sampler: Sampler,
    pub textures: FastCache<UVec2, RenderSprite>,
}

impl RenderSpriteCache {
    pub fn new(capacity: usize, sampler: Sampler) -> Result<Self, String> {
        let capacity = NonZeroUsize::new(capacity).ok_or("Capacity must be higher than 0")?;
        let textures = FastCache::new(capacity);
        Ok(Self { sampler, textures })
    }

    pub fn get(&mut self, size: UVec2) -> &RenderSprite {
        self.textures.get_or_insert(size, || {
            let render_texture = create_render_texture()
                .with_label(&format!("Cached RenderTexture({size})"))
                .with_size(size.x, size.y)
                .build()
                .unwrap();

            let sprite = create_sprite()
                .with_label(&format!("Cached Sprite({size})"))
                .with_sampler(&self.sampler)
                .from_texture(render_texture.texture())
                .with_write_flag(true)
                .build()
                .unwrap();

            RenderSprite {
                sprite,
                render_texture,
            }
        })
    }

    pub fn clear(&mut self) {
        self.textures.clear();
    }
}

#[derive(Clone)]
pub struct RenderSprite {
    pub sprite: Sprite,
    pub render_texture: RenderTexture,
}

#[derive(Default)]
pub struct RenderSpriteBuilder<'a> {
    rt_builder: RenderTextureBuilder<'a>,
    sprite_builder: SpriteBuilder<'a>,
}

impl<'a> RenderSpriteBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_label(mut self, label: &'a str) -> Self {
        self.rt_builder = self.rt_builder.with_label(label);
        self.sprite_builder = self.sprite_builder.with_label(label);
        self
    }

    #[inline]
    pub fn with_depth(mut self, enabled: bool) -> Self {
        self.rt_builder = self.rt_builder.with_depth(enabled);
        self
    }

    #[inline]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.rt_builder = self.rt_builder.with_size(width, height);
        self
    }

    #[inline]
    pub fn with_sampler(mut self, sampler: &Sampler) -> Self {
        self.sprite_builder = self.sprite_builder.with_sampler(sampler);
        self
    }

    #[inline]
    pub fn with_format(mut self, format: TextureFormat) -> Self {
        self.rt_builder = self.rt_builder.with_format(format);
        self
    }

    #[inline]
    pub fn with_write_flag(mut self, writable: bool) -> Self {
        self.sprite_builder = self.sprite_builder.with_write_flag(writable);
        self
    }

    #[inline]
    pub fn with_wrap_x(mut self, wrap: TextureWrap) -> Self {
        self.sprite_builder = self.sprite_builder.with_wrap_x(wrap);
        self
    }

    #[inline]
    pub fn with_wrap_y(mut self, wrap: TextureWrap) -> Self {
        self.sprite_builder = self.sprite_builder.with_wrap_y(wrap);
        self
    }

    #[inline]
    pub fn with_wrap_z(mut self, wrap: TextureWrap) -> Self {
        self.sprite_builder = self.sprite_builder.with_wrap_z(wrap);
        self
    }

    #[inline]
    pub fn with_min_filter(mut self, filter: TextureFilter) -> Self {
        self.sprite_builder = self.sprite_builder.with_min_filter(filter);
        self
    }

    #[inline]
    pub fn with_mag_filter(mut self, filter: TextureFilter) -> Self {
        self.sprite_builder = self.sprite_builder.with_mag_filter(filter);
        self
    }

    #[inline]
    pub fn with_mipmap_filter(mut self, filter: TextureFilter) -> Self {
        self.sprite_builder = self.sprite_builder.with_mipmap_filter(filter);
        self
    }

    #[inline]
    pub fn build(self) -> Result<RenderSprite, String> {
        let Self {
            rt_builder,
            sprite_builder,
        } = self;
        let render_texture = rt_builder.build()?;
        let sprite = sprite_builder
            .from_texture(render_texture.texture())
            .build()?;
        Ok(RenderSprite {
            sprite,
            render_texture,
        })
    }
}
