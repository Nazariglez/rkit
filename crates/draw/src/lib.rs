mod m2d;

pub use m2d::*;

use core::gfx::RenderPipeline;

// -- Draw API
#[inline]
pub fn add_2d_pipeline(id: &str, pip: RenderPipeline) -> Option<RenderPipeline> {
    get_mut_2d_painter().add_pipeline(id, pip)
}

pub fn remove_2d_pipeline(id: &str) -> Option<RenderPipeline> {
    get_mut_2d_painter().remove_pipeline(id)
}

#[inline]
pub fn draw_2d() -> Draw2D {
    Draw2D::new()
}
