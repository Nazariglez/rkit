use crate::backend::gfx::Texture;
use crate::gfx::consts::{
    MAX_SAMPLED_TEXTURES_PER_SHADER_STAGE, MAX_UNIFORM_BUFFERS_PER_SHADER_STAGE,
};
use crate::gfx::{BindGroupLayoutRef, Buffer, Sampler, TextureId};
use arrayvec::ArrayVec;

pub const MAX_BINDING_ENTRIES: usize =
    MAX_UNIFORM_BUFFERS_PER_SHADER_STAGE + MAX_SAMPLED_TEXTURES_PER_SHADER_STAGE;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct BindGroupId(pub(crate) u64);

impl From<u64> for BindGroupId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct BindGroupLayoutId(pub(crate) u64);

impl From<u64> for BindGroupLayoutId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Default)]
pub struct BindGroupLayout {
    pub entries: ArrayVec<BindingType, MAX_BINDING_ENTRIES>,
}

impl BindGroupLayout {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_entry(mut self, binding: BindingType) -> Self {
        debug_assert!(
            self.entries.len() < MAX_BINDING_ENTRIES,
            "Cannot set more than {MAX_BINDING_ENTRIES} entries in BindGroupLayout"
        );
        self.entries.push(binding);
        self
    }
}

#[derive(Copy, Clone)]
pub(crate) enum BindType {
    Texture,
    Sampler,
    Uniform,
}

#[derive(Copy, Clone)]
pub struct BindingType {
    pub(crate) location: u32,
    pub(crate) typ: BindType,
    pub(crate) visible_fragment: bool,
    pub(crate) visible_vertex: bool,
    pub(crate) visible_compute: bool,
}

impl BindingType {
    pub fn texture(location: u32) -> Self {
        Self {
            location,
            typ: BindType::Texture,
            visible_fragment: false,
            visible_vertex: false,
            visible_compute: false,
        }
    }

    pub fn sampler(location: u32) -> Self {
        Self {
            location,
            typ: BindType::Sampler,
            visible_fragment: false,
            visible_vertex: false,
            visible_compute: false,
        }
    }

    pub fn uniform(location: u32) -> Self {
        Self {
            location,
            typ: BindType::Uniform,
            visible_fragment: false,
            visible_vertex: false,
            visible_compute: false,
        }
    }

    pub fn with_fragment_visibility(mut self, visible: bool) -> Self {
        self.visible_fragment = visible;
        self
    }

    pub fn with_vertex_visibility(mut self, visible: bool) -> Self {
        self.visible_vertex = visible;
        self
    }

    pub fn with_compute_visibility(mut self, visible: bool) -> Self {
        self.visible_compute = visible;
        self
    }
}

#[derive(Default)]
pub struct BindGroupDescriptor<'a> {
    pub label: Option<&'a str>,
    pub layout: Option<&'a BindGroupLayoutRef>,
    pub entry: ArrayVec<BindGroupEntry<'a>, MAX_BINDING_ENTRIES>,
}

#[derive(Copy, Clone)]
pub enum BindGroupEntry<'a> {
    Texture { location: u32, texture: &'a Texture },
    Sampler { location: u32, sampler: &'a Sampler },
    Uniform { location: u32, buffer: &'a Buffer },
}
