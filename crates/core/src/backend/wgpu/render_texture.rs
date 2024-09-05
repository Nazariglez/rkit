use crate::backend::wgpu::texture::Texture;
use crate::gfx::RenderTextureId;
use crate::math::UVec2;
use std::ops::Deref;

#[derive(Clone, Debug)]
pub struct RenderTexture {
    pub(crate) id: RenderTextureId,
    pub(crate) texture: Texture,
    pub(crate) depth_texture: Option<Texture>,
}

impl RenderTexture {
    pub fn id(&self) -> RenderTextureId {
        self.id
    }

    pub fn size(&self) -> UVec2 {
        self.texture.size()
    }

    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    pub fn height(&self) -> u32 {
        self.texture.height()
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn into_inner(self) -> Texture {
        let Self { texture, .. } = self;
        texture
    }
}

impl AsRef<Texture> for RenderTexture {
    fn as_ref(&self) -> &Texture {
        &self.texture
    }
}

impl Deref for RenderTexture {
    type Target = Texture;

    fn deref(&self) -> &Self::Target {
        self.texture()
    }
}
