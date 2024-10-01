use super::consts::{
    MAX_BIND_GROUPS_PER_PIPELINE, MAX_PIPELINE_COMPATIBLE_TEXTURES, MAX_VERTEX_BUFFERS,
};
use crate::gfx::{BindGroupLayout, BlendMode, Color, IndexFormat, TextureFormat, VertexLayout};
use arrayvec::ArrayVec;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct PipelineId(pub(crate) u64);

impl From<u64> for PipelineId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Default, Clone)]
pub struct RenderPipelineDescriptor<'a> {
    pub label: Option<&'a str>,
    pub shader: &'a str,
    pub depth_stencil: Option<DepthStencil>,
    pub stencil: Option<Stencil>,
    pub vertex_layout: ArrayVec<VertexLayout, MAX_VERTEX_BUFFERS>,
    pub primitive: Primitive,
    pub index_format: IndexFormat,
    pub bind_group_layout: ArrayVec<BindGroupLayout, MAX_BIND_GROUPS_PER_PIPELINE>,
    pub blend_mode: Option<BlendMode>,
    pub cull_mode: Option<CullMode>,
    pub vs_entry: Option<&'a str>,
    pub fs_entry: Option<&'a str>,
    pub color_mask: ColorMask,
    pub compatible_textures: ArrayVec<TextureFormat, MAX_PIPELINE_COMPATIBLE_TEXTURES>,
}

#[derive(Debug, Copy, Clone)]
pub struct DepthStencil {
    pub write: bool,
    pub compare: CompareMode,
}

#[derive(Default, Debug, Copy, Clone)]
pub enum Primitive {
    Points,
    Lines,
    LineStrip,
    #[default]
    Triangles,
    TriangleStrip,
}

#[derive(Debug, Copy, Clone)]
pub enum CullMode {
    Front,
    Back,
}

/// Clear options to use at the beginning of the frame
#[derive(Default, Debug, Clone, Copy)]
pub struct ClearOptions {
    pub color: Option<Color>,
    pub depth: Option<f32>,
    pub stencil: Option<u32>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CompareMode {
    Never,
    Less,
    Equal,
    LEqual,
    Greater,
    NotEqual,
    GEqual,
    Always,
}

/// Represent's the stencil action
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum StencilAction {
    Keep,
    Zero,
    Replace,
    Increment,
    IncrementWrap,
    Decrement,
    DecrementWrap,
    Invert,
}

/// Represents the stencil's option
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Stencil {
    pub stencil_fail: StencilAction,
    pub depth_fail: StencilAction,
    pub pass: StencilAction,
    pub compare: CompareMode,
    pub read_mask: u32,
    pub write_mask: u32,
    pub reference: u8,
}

/// Represents the color mask
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ColorMask {
    pub r: bool,
    pub g: bool,
    pub b: bool,
    pub a: bool,
}

impl Default for ColorMask {
    fn default() -> Self {
        Self {
            r: true,
            g: true,
            b: true,
            a: true,
        }
    }
}

impl ColorMask {
    pub const ALL: ColorMask = ColorMask {
        r: true,
        g: true,
        b: true,
        a: true,
    };

    pub const NONE: ColorMask = ColorMask {
        r: false,
        g: false,
        b: false,
        a: false,
    };
}
