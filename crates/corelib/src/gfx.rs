mod bind_group;
mod blend_mode;
mod buffer;
mod builders;
mod color;
mod compute;
pub mod consts;
mod limits;
mod pipeline;
mod readback;
mod renderer;
mod shader;
mod stats;
mod texture;

pub use crate::backend::gfx::*;
use crate::backend::{BackendImpl, GfxBackendImpl, get_mut_backend};
pub use bind_group::*;
pub use blend_mode::*;
pub use buffer::*;
pub use builders::*;
pub use color::*;
pub use compute::*;
pub use limits::*;
pub use pipeline::*;
pub use readback::*;
pub use renderer::*;
pub use shader::*;
pub use stats::*;
pub use texture::*;

// - Gfx
#[inline]
pub fn frame_size() -> crate::math::UVec2 {
    get_mut_backend().gfx().frame_size()
}

#[inline]
pub fn render_to_frame<R>(renderer: &R) -> Result<(), String>
where
    R: AsRenderer,
{
    renderer.render(None)
}

#[inline]
pub fn render_to_texture<R>(texture: &RenderTexture, renderer: &R) -> Result<(), String>
where
    R: AsRenderer,
{
    renderer.render(Some(texture))
}

#[inline]
pub fn create_shader(source: &str) -> ShaderBuilder<'_> {
    ShaderBuilder::new(source)
}

#[inline]
pub fn create_render_pipeline(source: &str) -> RenderPipelineBuilder<'_> {
    RenderPipelineBuilder::new(ShaderInput::Source(source))
}

#[inline]
pub fn create_compute_pipeline<'a>(
    input: impl Into<ShaderInput<'a>>,
) -> ComputePipelineBuilder<'a> {
    ComputePipelineBuilder::new(input.into())
}

#[doc(hidden)]
pub fn create_stencil_variant(
    base: &RenderPipeline,
    stencil: Stencil,
) -> Result<RenderPipeline, String> {
    get_mut_backend()
        .gfx()
        .create_stencil_variant(base, stencil)
}

#[inline]
pub fn create_vertex_buffer<D: bytemuck::Pod>(data: &[D]) -> BufferBuilder<'_> {
    BufferBuilder::new(BufferUsage::Vertex, data)
}

#[inline]
pub fn create_index_buffer<D: bytemuck::Pod>(data: &[D]) -> BufferBuilder<'_> {
    BufferBuilder::new(BufferUsage::Index, data)
}

#[inline]
pub fn create_uniform_buffer<D: bytemuck::Pod>(data: &[D]) -> BufferBuilder<'_> {
    BufferBuilder::new(BufferUsage::Uniform, data)
}

#[inline]
pub fn create_storage_buffer<D: bytemuck::Pod>(data: &[D]) -> BufferBuilder<'_> {
    BufferBuilder::new(BufferUsage::Storage, data)
}

#[inline]
pub fn create_packed_storage_buffer<T: StorageData>(
    values: &[T],
) -> PackedStorageBufferBuilder<'_, T> {
    PackedStorageBufferBuilder::new(values)
}

#[inline]
pub fn create_indirect_buffer<A: IndirectArgs>() -> IndirectBufferBuilder<A> {
    IndirectBufferBuilder::new()
}

#[inline]
pub fn create_bind_group<'a>() -> BindGroupBuilder<'a> {
    BindGroupBuilder::new()
}

#[inline]
pub fn write_buffer(buffer: &Buffer) -> BufferWriteBuilder<'_> {
    BufferWriteBuilder::new(buffer)
}

#[inline]
pub fn read_buffer(buffer: &Buffer) -> BufferReadbackBuilder<'_> {
    BufferReadbackBuilder::new(buffer)
}

#[inline]
pub fn read_texture(texture: &Texture) -> TextureReadbackBuilder<'_> {
    TextureReadbackBuilder::new(texture)
}

#[inline]
pub fn compute(compute: &Compute<'_>) -> Result<(), String> {
    get_mut_backend().gfx().compute(compute)
}

#[inline]
pub fn write_texture(tex: &Texture) -> TextureWriteBuilder<'_> {
    TextureWriteBuilder::new(tex)
}

#[inline]
pub fn generate_mipmaps(texture: &Texture) -> Result<(), String> {
    get_mut_backend().gfx().generate_mipmaps(texture)
}

#[inline]
pub fn create_sampler<'a>() -> SamplerBuilder<'a> {
    SamplerBuilder::new()
}

#[inline]
pub fn create_texture<'a>() -> TextureBuilder<'a> {
    TextureBuilder::new()
}

#[inline]
pub fn create_storage_texture<'a>() -> TextureBuilder<'a> {
    TextureBuilder::storage()
}

#[inline]
pub fn create_render_texture<'a>() -> RenderTextureBuilder<'a> {
    RenderTextureBuilder::new()
}

#[inline]
pub fn limits() -> Limits {
    get_mut_backend().gfx().limits()
}

#[inline]
pub fn last_frame_stats() -> GpuStats {
    get_mut_backend().gfx().stats()
}
