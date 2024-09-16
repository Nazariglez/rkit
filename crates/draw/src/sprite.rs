use core::gfx::{
    Sampler, SamplerBuilder, SamplerId, Texture, TextureBuilder, TextureFilter, TextureFormat,
    TextureId, TextureWrap,
};
use core::math::{UVec2, Vec2};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct SpriteId {
    texture: TextureId,
    sampler: SamplerId,
}

#[derive(Clone)]
pub struct Sprite {
    texture: Texture,
    sampler: Sampler,
    //frame: RectFrame, // TODO sprite rect frame
    pub(crate) expired_signal: Arc<AtomicBool>,
}

impl PartialEq for Sprite {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Sprite {
    pub fn id(&self) -> SpriteId {
        SpriteId {
            texture: self.texture.id(),
            sampler: self.sampler.id(),
        }
    }
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    pub fn size(&self) -> Vec2 {
        self.texture.size()
    }
}

impl Drop for Sprite {
    fn drop(&mut self) {
        self.expired_signal.store(true, Ordering::SeqCst);
    }
}

pub struct SpriteBuilder<'a> {
    texture_builder: TextureBuilder<'a>,
    sampler_builder: SamplerBuilder<'a>,
    texture: Option<Texture>,
    sampler: Option<Sampler>,
}

impl<'a> SpriteBuilder<'a> {
    pub fn new() -> Self {
        Self {
            texture_builder: TextureBuilder::new(),
            sampler_builder: SamplerBuilder::new(),
            texture: None,
            sampler: None,
        }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.texture_builder = self.texture_builder.with_label(label);
        self.sampler_builder = self.sampler_builder.with_label(label);
        self
    }

    pub fn from_texture(mut self, tex: &Texture) -> Self {
        self.texture = Some(tex.clone());
        self
    }

    pub fn from_image(mut self, image: &'a [u8]) -> Self {
        self.texture_builder = self.texture_builder.from_image(image);
        self
    }

    pub fn with_sampler(mut self, sampler: Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    pub fn with_format(mut self, format: TextureFormat) -> Self {
        self.texture_builder = self.texture_builder.with_format(format);
        self
    }

    pub fn with_write_flag(mut self, writable: bool) -> Self {
        self.texture_builder = self.texture_builder.with_write_flag(writable);
        self
    }

    pub fn with_wrap_x(mut self, wrap: TextureWrap) -> Self {
        self.sampler_builder = self.sampler_builder.with_wrap_x(wrap);
        self
    }

    pub fn with_wrap_y(mut self, wrap: TextureWrap) -> Self {
        self.sampler_builder = self.sampler_builder.with_wrap_y(wrap);
        self
    }

    pub fn with_wrap_z(mut self, wrap: TextureWrap) -> Self {
        self.sampler_builder = self.sampler_builder.with_wrap_z(wrap);
        self
    }

    pub fn with_min_filter(mut self, filter: TextureFilter) -> Self {
        self.sampler_builder = self.sampler_builder.with_min_filter(filter);
        self
    }

    pub fn with_mag_filter(mut self, filter: TextureFilter) -> Self {
        self.sampler_builder = self.sampler_builder.with_mag_filter(filter);
        self
    }

    pub fn with_mipmap_filter(mut self, filter: TextureFilter) -> Self {
        self.sampler_builder = self.sampler_builder.with_mipmap_filter(filter);
        self
    }

    pub fn build(self) -> Result<Sprite, String> {
        let Self {
            texture_builder,
            sampler_builder,
            texture,
            sampler,
        } = self;
        let texture = match texture {
            None => texture_builder.build()?,
            Some(t) => t,
        };
        let sampler = match sampler {
            None => sampler_builder.build()?,
            Some(s) => s,
        };
        let expired_signal = Arc::new(AtomicBool::new(false));
        // TODO frame
        Ok(Sprite {
            texture,
            sampler,
            expired_signal,
        })
    }
}
