mod m2d;
mod shapes;
mod sprite;
pub mod text;

pub use m2d::*;
pub use sprite::*;

pub use text::*;

use core::app::window_size;

// -- Draw API
#[inline]
pub fn create_sprite<'a>() -> SpriteBuilder<'a> {
    SpriteBuilder::new()
}

#[inline]
pub fn add_pipeline_2d<F: FnOnce(PipelineResources<'_>) -> PipelineContext>(
    cb: F,
) -> DrawPipelineId {
    let mut painter = get_mut_2d_painter();
    let ctx = cb(painter.pip_resources());
    painter.add_pipeline(ctx)
}

#[inline]
pub fn set_pipeline_2d<F: FnOnce(PipelineResources<'_>) -> PipelineContext>(
    id: &DrawPipelineId,
    cb: F,
) -> Option<PipelineContext> {
    let mut painter = get_mut_2d_painter();
    let ctx = cb(painter.pip_resources());
    painter.set_pipeline(id, ctx)
}

#[inline]
pub fn remove_pipeline_2d(id: &DrawPipelineId) -> Option<PipelineContext> {
    get_mut_2d_painter().remove_pipeline(id)
}

#[inline]
pub fn create_draw_2d() -> Draw2D {
    Draw2D::new(window_size())
}

#[inline]
pub(crate) fn clean_2d() {
    get_mut_2d_painter().clean();
}

// -- text
pub struct FontBuilder<'a> {
    source: &'a [u8],
    nearest: bool,
}

impl<'a> FontBuilder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            source: data,
            nearest: false,
        }
    }

    pub fn with_nearest_filter(mut self, nearest: bool) -> Self {
        self.nearest = nearest;
        self
    }

    // TODO from_system("Arial") it uses a system font (not supported on wasm)

    pub fn build(self) -> Result<Font, String> {
        let Self { source, nearest } = self;
        get_mut_text_system().create_font(source, nearest)
    }
}

#[inline]
pub fn create_font(data: &[u8]) -> FontBuilder {
    FontBuilder::new(data)
}

#[inline]
pub fn set_default_font(font: &Font) {
    get_mut_text_system().set_default_font(font);
}
