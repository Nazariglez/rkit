use strum_macros::EnumCount;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct TextureId(pub(crate) u64);

impl From<u64> for TextureId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<TextureId> for u64 {
    fn from(value: TextureId) -> Self {
        value.0
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct TextureDescriptor<'a> {
    pub label: Option<&'a str>,
    pub format: TextureFormat,
    pub write: bool,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct TextureData<'a> {
    pub bytes: &'a [u8],
    pub width: u32,
    pub height: u32,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct RenderTextureId(pub(crate) u64);

impl From<u64> for RenderTextureId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct RenderTextureDescriptor<'a> {
    pub label: Option<&'a str>,
    pub depth: bool,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

#[derive(Debug, Default, Copy, Clone)]
pub enum TextureWrap {
    #[default]
    Clamp,
    Repeat,
    MirrorRepeat,
}

#[derive(Debug, Default, Copy, Clone)]
pub enum TextureFilter {
    #[default]
    Linear,
    Nearest,
}

/// Enum representing texture formats supported by WebGL2
/// which is the min compatibility layer we aim for
#[derive(Debug, Copy, Clone, EnumCount)]
pub enum TextureFormat {
    // Single channel 8-bit textures
    R8UNorm, // WebGL2: GL_R8
    R8INorm, // WebGL2: GL_R8_SNORM
    R8UInt,  // WebGL2: GL_R8UI
    R8Int,   // WebGL2: GL_R8I

    // Two channel 8-bit textures
    Rg8UNorm, // WebGL2: GL_RG8
    Rg8INorm, // WebGL2: GL_RG8_SNORM
    Rg8UInt,  // WebGL2: GL_RG8UI
    Rg8Int,   // WebGL2: GL_RG8I

    // Four channel 8-bit textures
    Rgba8UNorm,     // WebGL2: GL_RGBA8
    Rgba8UNormSrgb, // WebGL2: GL_SRGB8_ALPHA8
    Rgba8INorm,     // WebGL2: GL_RGBA8_SNORM
    Rgba8UInt,      // WebGL2: GL_RGBA8UI
    Rgba8Int,       // WebGL2: GL_RGBA8I
    Bgra8UNorm,     // WebGL2: GL_BGRA8_EXT (via EXT_texture_format_BGRA8888 extension)
    Bgra8UNormSrgb, // WebGL2: not supported

    // Single channel 16-bit textures
    R16UNorm, // WebGL2: GL_R16 (requires EXT_texture_norm16 extension)
    R16INorm, // WebGL2: GL_R16_SNORM (requires EXT_texture_norm16 extension)
    R16UInt,  // WebGL2: GL_R16UI
    R16Int,   // WebGL2: GL_R16I
    R16Float, // WebGL2: GL_R16F

    // Two channel 16-bit textures
    Rg16UNorm, // WebGL2: GL_RG16 (requires EXT_texture_norm16 extension)
    Rg16INorm, // WebGL2: GL_RG16_SNORM (requires EXT_texture_norm16 extension)
    Rg16UInt,  // WebGL2: GL_RG16UI
    Rg16Int,   // WebGL2: GL_RG16I
    Rg16Float, // WebGL2: GL_RG16F

    // Four channel 16-bit textures
    Rgba16UNorm, // WebGL2: GL_RGBA16 (requires EXT_texture_norm16 extension)
    Rgba16INorm, // WebGL2: GL_RGBA16_SNORM (requires EXT_texture_norm16 extension)
    Rgba16UInt,  // WebGL2: GL_RGBA16UI
    Rgba16Int,   // WebGL2: GL_RGBA16I
    Rgba16Float, // WebGL2: GL_RGBA16F

    // Single channel 32-bit textures
    R32UInt,  // WebGL2: GL_R32UI
    R32Int,   // WebGL2: GL_R32I
    R32Float, // WebGL2: GL_R32F

    // Two channel 32-bit textures
    Rg32UInt,  // WebGL2: GL_RG32UI
    Rg32Int,   // WebGL2: GL_RG32I
    Rg32Float, // WebGL2: GL_RG32F

    // Four channel 32-bit textures
    Rgba32UInt,  // WebGL2: GL_RGBA32UI
    Rgba32Int,   // WebGL2: GL_RGBA32I
    Rgba32Float, // WebGL2: GL_RGBA32F

    // Depth and stencil formats
    Depth16,              // WebGL2: GL_DEPTH_COMPONENT16
    Depth24, // WebGL2: GL_DEPTH_COMPONENT24 (might be implemented using GL_DEPTH24_STENCIL8)
    Depth32Float, // WebGL2: GL_DEPTH_COMPONENT32F
    Depth24Stencil8, // WebGL2: GL_DEPTH24_STENCIL8
    Depth32FloatStencil8, // WebGL2: GL_DEPTH32F_STENCIL8 (via WEBGL_depth_texture extension)
}

impl TextureFormat {
    #[inline]
    pub fn is_srgb(&self) -> bool {
        matches!(
            self,
            TextureFormat::Rgba8UNormSrgb | TextureFormat::Bgra8UNormSrgb
        )
    }

    #[inline]
    pub fn channels(&self) -> u8 {
        match self {
            TextureFormat::R8UNorm
            | TextureFormat::R8INorm
            | TextureFormat::R8UInt
            | TextureFormat::R8Int
            | TextureFormat::R16UNorm
            | TextureFormat::R16INorm
            | TextureFormat::R16UInt
            | TextureFormat::R16Int
            | TextureFormat::R16Float
            | TextureFormat::R32UInt
            | TextureFormat::R32Int
            | TextureFormat::R32Float => 1,

            TextureFormat::Rg8UNorm
            | TextureFormat::Rg8INorm
            | TextureFormat::Rg8UInt
            | TextureFormat::Rg8Int
            | TextureFormat::Rg16UNorm
            | TextureFormat::Rg16INorm
            | TextureFormat::Rg16UInt
            | TextureFormat::Rg16Int
            | TextureFormat::Rg16Float
            | TextureFormat::Rg32UInt
            | TextureFormat::Rg32Int
            | TextureFormat::Rg32Float => 2,

            TextureFormat::Rgba8UNorm
            | TextureFormat::Rgba8UNormSrgb
            | TextureFormat::Rgba8INorm
            | TextureFormat::Rgba8UInt
            | TextureFormat::Rgba8Int
            | TextureFormat::Bgra8UNorm
            | TextureFormat::Bgra8UNormSrgb
            | TextureFormat::Rgba16UNorm
            | TextureFormat::Rgba16INorm
            | TextureFormat::Rgba16UInt
            | TextureFormat::Rgba16Int
            | TextureFormat::Rgba16Float
            | TextureFormat::Rgba32UInt
            | TextureFormat::Rgba32Int
            | TextureFormat::Rgba32Float => 4,

            // TODO, is this right? depth textures will ever need to know how many channels?
            TextureFormat::Depth16
            | TextureFormat::Depth24
            | TextureFormat::Depth32Float
            | TextureFormat::Depth24Stencil8
            | TextureFormat::Depth32FloatStencil8 => 0,
        }
    }
}

impl Default for TextureFormat {
    fn default() -> Self {
        // TODO this could be different depending on platforms (webgl2?)
        Self::Rgba8UNormSrgb
    }
}

// - Sampler
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct SamplerId(pub(crate) u64);

impl From<u64> for SamplerId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct SamplerDescriptor<'a> {
    pub label: Option<&'a str>,
    pub wrap_x: TextureWrap,
    pub wrap_y: TextureWrap,
    pub wrap_z: TextureWrap,
    pub mag_filter: TextureFilter,
    pub min_filter: TextureFilter,
    pub mipmap_filter: Option<TextureFilter>,
}
