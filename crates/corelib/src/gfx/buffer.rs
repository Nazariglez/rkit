use super::consts::MAX_VERTEX_ATTRIBUTES;
use crate::backend::{BackendImpl, GfxBackendImpl, get_mut_backend, gfx::Buffer};
use arrayvec::ArrayVec;
use std::{marker::PhantomData, ops::Deref};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BufferId(pub(crate) u64);

impl From<u64> for BufferId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct BufferDescriptor<'a> {
    pub label: Option<&'a str>,
    pub usage: BufferUsage,
    pub content: &'a [u8],
    pub write: bool,
    pub(crate) allocation_size: Option<usize>,
    pub(crate) indirect: bool,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum BufferWriteMode {
    Immediate,
    Ordered,
}

pub(crate) fn validate_buffer_write(
    buffer: &Buffer,
    offset: u64,
    size: usize,
    mode: BufferWriteMode,
) -> Result<usize, String> {
    if !buffer.is_writable() {
        return Err("Buffer is not writable".to_string());
    }
    if offset % 4 != 0 {
        return Err("Buffer write offsets must be divisible by four bytes".to_string());
    }
    if size % 4 != 0 {
        return Err("Buffer write sizes must be divisible by four bytes".to_string());
    }

    let offset = usize::try_from(offset)
        .map_err(|_| "Buffer write offset does not fit this platform".to_string())?;
    let end = offset
        .checked_add(size)
        .ok_or_else(|| "Buffer write size overflows".to_string())?;
    let fixed_allocation = mode == BufferWriteMode::Ordered
        || matches!(buffer.usage(), BufferUsage::Uniform | BufferUsage::Storage);
    if fixed_allocation && end > buffer.size() {
        return Err(
            "Buffer update exceeds its fixed allocation; create a replacement buffer and rebuild its bind groups"
                .to_string(),
        );
    }
    Ok(end)
}

pub(crate) fn validate_buffer_range(
    buffer: &Buffer,
    bytes: std::ops::Range<u64>,
    operation: &str,
) -> Result<(), String> {
    if bytes.start >= bytes.end {
        return Err(format!("{operation} ranges must be nonempty"));
    }
    if bytes.start % 4 != 0 || bytes.end % 4 != 0 {
        return Err(format!(
            "{operation} ranges must be divisible by four bytes"
        ));
    }
    let size = u64::try_from(buffer.size())
        .map_err(|_| "Buffer size does not fit the supported range".to_string())?;
    if bytes.end > size {
        return Err(format!("{operation} range exceeds the allocation"));
    }
    Ok(())
}

pub(crate) fn validate_buffer_clear(
    buffer: &Buffer,
    bytes: std::ops::Range<u64>,
) -> Result<(), String> {
    if buffer.usage() != BufferUsage::Storage {
        return Err("Buffer clears require a storage buffer".to_string());
    }
    validate_buffer_range(buffer, bytes, "Buffer clear")
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub enum BufferUsage {
    #[default]
    Vertex,
    Index,
    Uniform,
    Storage,
}

pub trait StorageData:
    encase::ShaderType + encase::ShaderSize + encase::internal::WriteInto
{
}

impl<T> StorageData for T where
    T: encase::ShaderType + encase::ShaderSize + encase::internal::WriteInto
{
}

pub struct PackedStorageBuffer<T> {
    buffer: Buffer,
    marker: PhantomData<T>,
}

impl<T> PackedStorageBuffer<T> {
    pub(crate) fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            marker: PhantomData,
        }
    }
}

impl<T> Clone for PackedStorageBuffer<T> {
    fn clone(&self) -> Self {
        Self::new(self.buffer.clone())
    }
}

impl<T: StorageData> PackedStorageBuffer<T> {
    pub fn update(&self, values: &[T]) -> Result<(), String> {
        let size = storage_byte_len::<T>(values.len())?;
        validate_buffer_write(&self.buffer, 0, size, BufferWriteMode::Immediate)?;
        let bytes = encode_storage_values(values)?;
        get_mut_backend()
            .gfx()
            .write_buffer(&self.buffer, 0, &bytes)
    }
}

impl<T> Deref for PackedStorageBuffer<T> {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<T> AsRef<Buffer> for PackedStorageBuffer<T> {
    fn as_ref(&self) -> &Buffer {
        &self.buffer
    }
}

pub(crate) fn storage_stride<T: StorageData>() -> Result<usize, String> {
    let size = encase::internal::SizeValue::from(T::SHADER_SIZE);
    let stride = T::METADATA.alignment().round_up_size(size).get();
    usize::try_from(stride)
        .map_err(|_| "Packed storage stride does not fit this platform".to_string())
}

pub(crate) fn storage_byte_len<T: StorageData>(count: usize) -> Result<usize, String> {
    storage_stride::<T>()?
        .checked_mul(count)
        .ok_or_else(|| "Packed storage allocation size overflows".to_string())
}

pub(crate) fn encode_storage_values<T: StorageData>(values: &[T]) -> Result<Vec<u8>, String> {
    let size = storage_byte_len::<T>(values.len())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| "Packed storage encoding allocation failed".to_string())?;
    bytes.resize(size, 0);
    let stride = storage_stride::<T>()?;
    for (index, value) in values.iter().enumerate() {
        let offset = stride
            .checked_mul(index)
            .ok_or_else(|| "Packed storage offset overflows".to_string())?;
        let mut writer = encase::internal::Writer::new(value, &mut bytes, offset)
            .map_err(|error| format!("Cannot encode packed storage value: {error}"))?;
        value.write_into(&mut writer);
    }
    Ok(bytes)
}

mod indirect_args {
    pub trait Sealed {}
}

pub trait IndirectArgs: indirect_args::Sealed + bytemuck::Pod {
    #[doc(hidden)]
    const BYTE_SIZE: usize;
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub struct DispatchArgs {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub struct DrawArgs {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

impl indirect_args::Sealed for DispatchArgs {}
impl IndirectArgs for DispatchArgs {
    const BYTE_SIZE: usize = 12;
}

impl indirect_args::Sealed for DrawArgs {}
impl IndirectArgs for DrawArgs {
    const BYTE_SIZE: usize = 16;
}

pub struct IndirectBuffer<A> {
    buffer: Buffer,
    marker: PhantomData<A>,
}

impl<A> IndirectBuffer<A> {
    pub(crate) fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            marker: PhantomData,
        }
    }
}

impl<A> Clone for IndirectBuffer<A> {
    fn clone(&self) -> Self {
        Self::new(self.buffer.clone())
    }
}

impl<A> Deref for IndirectBuffer<A> {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<A> AsRef<Buffer> for IndirectBuffer<A> {
    fn as_ref(&self) -> &Buffer {
        &self.buffer
    }
}

#[derive(Default, Debug, Clone)]
pub struct VertexLayout {
    pub step_mode: VertexStepMode,
    pub attributes: ArrayVec<VertexAttribute, MAX_VERTEX_ATTRIBUTES>,
}

impl VertexLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_step_mode(mut self, step_mode: VertexStepMode) -> Self {
        self.step_mode = step_mode;
        self
    }

    pub fn with_attr(mut self, location: u32, format: VertexFormat) -> Self {
        debug_assert!(
            self.attributes.len() < MAX_VERTEX_ATTRIBUTES,
            "Cannot set more than {MAX_VERTEX_ATTRIBUTES} attributes in VertexLayout"
        );
        self.attributes.push(VertexAttribute { location, format });
        self
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub enum VertexStepMode {
    #[default]
    Vertex,
    Instance,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct VertexAttribute {
    pub location: u32,
    pub format: VertexFormat,
}

#[derive(Default, Debug, Copy, Clone)]
pub enum VertexFormat {
    #[default]
    UInt8x2,
    UInt8x4,
    Int8x2,
    Int8x4,
    U8x2norm,
    U8x4norm,
    I8x2norm,
    I8x4norm,
    UInt16x2,
    UInt16x4,
    Int16x2,
    Int16x4,
    U16x2norm,
    U16x4norm,
    Int16x2norm,
    Int16x4norm,
    Float16x2,
    Float16x4,
    Float32,
    Float32x2,
    Float32x3,
    Float32x4,
    UInt32,
    UInt32x2,
    UInt32x3,
    UInt32x4,
    Int32,
    Int32x2,
    Int32x3,
    Int32x4,
}

#[derive(Debug, Default, Copy, Clone)]
pub enum IndexFormat {
    UInt16,
    #[default]
    UInt32,
}
