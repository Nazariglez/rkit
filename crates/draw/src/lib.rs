mod m2d;
mod sprite;

pub use m2d::*;
pub use sprite::*;

// -- Draw API
#[inline]
pub fn create_sprite<'a>() -> SpriteBuilder<'a> {
    SpriteBuilder::new()
}

#[inline]
pub fn add_2d_pipeline(id: &str, pip: PipelineContext) -> Option<PipelineContext> {
    get_mut_2d_painter().add_pipeline(id, pip)
}

pub fn remove_2d_pipeline(id: &str) -> Option<PipelineContext> {
    get_mut_2d_painter().remove_pipeline(id)
}

#[inline]
pub fn draw_2d() -> Draw2D {
    Draw2D::new()
}

// TODO execute this somehow at the end of the frame
#[inline]
pub fn clean_2d() {
    get_mut_2d_painter().clean();
}
