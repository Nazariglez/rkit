use crate::create_sprite;

use super::sprite::Sprite;
use corelib::gfx::{create_render_texture, RenderTexture, Sampler};
use corelib::math::UVec2;
use std::num::NonZeroUsize;
use utils::fast_cache::FastCache;

#[derive(Clone)]
pub struct RenderSprite {
    pub sprite: Sprite,
    pub render_texture: RenderTexture,
}

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
        let sprite = self.textures.get_or_insert(size, || {
            let render_texture = create_render_texture()
                .with_label(&format!("Cached RenderTexture({})", size))
                .with_size(size.x, size.y)
                .build()
                .unwrap();

            let sprite = create_sprite()
                .with_label(&format!("Cached Sprite({})", size))
                .with_sampler(&self.sampler)
                .from_texture(render_texture.texture())
                .with_write_flag(true)
                .build()
                .unwrap();

            RenderSprite {
                sprite,
                render_texture,
            }
        });
        sprite
    }

    pub fn clear(&mut self) {
        self.textures.clear();
    }
}
