mod bind_group;
mod blend_mode;
mod buffer;
mod builders;
mod color;
pub mod consts;
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
pub use pipeline::*;
pub use pipeline::*;
pub use renderer::*;
pub use texture::*;

// - Gfx
#[inline]
pub fn render_to_frame(renderer: &Renderer) -> Result<(), String> {
    get_mut_backend().gfx().render(renderer)
}

#[inline]
pub fn render_to_texture(texture: &RenderTexture, renderer: &Renderer) -> Result<(), String> {
    get_mut_backend().gfx().render_to(texture, renderer)
}

#[inline]
pub fn create_render_pipeline(shader: &str) -> RenderPipelineBuilder {
    RenderPipelineBuilder::new(shader)
}

#[inline]
pub fn create_vertex_buffer<D: bytemuck::Pod>(data: &[D]) -> BufferBuilder {
    BufferBuilder::new(BufferUsage::Vertex, data)
}
