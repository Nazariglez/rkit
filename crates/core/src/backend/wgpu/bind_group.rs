use crate::gfx::{BindGroupId, BindGroupLayoutId};
use std::sync::Arc;
use wgpu::{BindGroup as RawBindGroup, BindGroupLayout};

#[derive(Clone)]
pub struct BindGroup {
    pub(crate) id: BindGroupId,
    pub(crate) raw: Arc<RawBindGroup>,
}

#[derive(Clone)]
pub struct BindGroupLayoutRef {
    pub(crate) id: BindGroupLayoutId,
    pub(crate) raw: Arc<BindGroupLayout>,
}
