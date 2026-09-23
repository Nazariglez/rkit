use crate::{
    backend::gfx::{Buffer, Texture},
    gfx::{BindGroupId, BindGroupLayoutId, BindingType, MAX_BINDING_ENTRIES, StorageTextureAccess},
};
use arrayvec::ArrayVec;
use std::sync::Arc;
use wgpu::{BindGroup as RawBindGroup, BindGroupLayout};

#[derive(Clone)]
pub(crate) struct BufferBinding {
    pub(crate) location: u32,
    pub(crate) size: u64,
    pub(crate) buffer: Buffer,
    pub(crate) writable: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum TextureBindingAccess {
    Sampled,
    Storage(StorageTextureAccess),
}

impl TextureBindingAccess {
    pub(crate) fn writable(self) -> bool {
        matches!(
            self,
            Self::Storage(StorageTextureAccess::Writeonly | StorageTextureAccess::Readwrite)
        )
    }

    pub(crate) fn conflicts_with(self, other: Self) -> bool {
        self != other && (self.writable() || other.writable())
    }
}

#[derive(Clone)]
pub(crate) struct TextureBinding {
    pub(crate) texture: Texture,
    pub(crate) access: TextureBindingAccess,
}

pub(crate) struct BindGroupInner {
    pub(crate) id: BindGroupId,
    pub(crate) layout: BindGroupLayoutId,
    pub(crate) raw: RawBindGroup,
    pub(crate) buffers: ArrayVec<BufferBinding, MAX_BINDING_ENTRIES>,
    pub(crate) textures: ArrayVec<TextureBinding, MAX_BINDING_ENTRIES>,
}

#[derive(Clone)]
pub struct BindGroup {
    pub(crate) inner: Arc<BindGroupInner>,
}

impl PartialEq for BindGroup {
    fn eq(&self, other: &Self) -> bool {
        self.inner.id == other.inner.id
    }
}

impl BindGroup {
    pub fn id(&self) -> BindGroupId {
        self.inner.id
    }
}

#[derive(Clone)]
pub struct BindGroupLayoutRef {
    pub(crate) id: BindGroupLayoutId,
    pub(crate) raw: Arc<BindGroupLayout>,
    pub(crate) entries: ArrayVec<BindingType, MAX_BINDING_ENTRIES>,
}

impl BindGroupLayoutRef {
    pub fn id(&self) -> BindGroupLayoutId {
        self.id
    }
}
