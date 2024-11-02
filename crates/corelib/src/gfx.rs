mod bind_group;
mod blend_mode;
mod buffer;
mod builders;
mod color;
pub mod consts;
mod limits;
mod pipeline;
mod renderer;
mod texture;

pub use crate::backend::gfx::*;
use crate::backend::{get_mut_backend, BackendImpl, GfxBackendImpl};
pub use bind_group::*;
pub use blend_mode::*;
pub use buffer::*;
pub use builders::*;
pub use color::*;
pub use limits::*;
pub use pipeline::*;
pub use renderer::*;
pub use texture::*;

// - Gfx
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
pub fn create_render_pipeline(shader: &str) -> RenderPipelineBuilder {
    RenderPipelineBuilder::new(shader)
}

#[inline]
pub fn create_vertex_buffer<D: bytemuck::Pod>(data: &[D]) -> BufferBuilder {
    BufferBuilder::new(BufferUsage::Vertex, data)
}

#[inline]
pub fn create_index_buffer<D: bytemuck::Pod>(data: &[D]) -> BufferBuilder {
    BufferBuilder::new(BufferUsage::Index, data)
}

#[inline]
pub fn create_uniform_buffer<D: bytemuck::Pod>(data: &[D]) -> BufferBuilder {
    BufferBuilder::new(BufferUsage::Uniform, data)
}

#[inline]
pub fn create_bind_group<'a>() -> BindGroupBuilder<'a> {
    BindGroupBuilder::new()
}

#[inline]
pub fn write_buffer(buffer: &Buffer) -> BufferWriteBuilder {
    BufferWriteBuilder::new(buffer)
}

#[inline]
pub fn write_texture(tex: &Texture) -> TextureWriteBuilder {
    TextureWriteBuilder::new(tex)
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
pub fn create_render_texture<'a>() -> RenderTextureBuilder<'a> {
    RenderTextureBuilder::new()
}

#[inline]
pub fn limits() -> Limits {
    get_mut_backend().gfx().limits()
}
