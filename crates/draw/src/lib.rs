mod m2d;
mod render_sprite_cache;
mod shapes;
mod sprite;

mod atlas;
pub mod text;

pub use atlas::*;
pub use m2d::*;
pub use render_sprite_cache::*;
pub use sprite::*;

pub use text::*;

use corelib::{
    app::window_size,
    gfx::{self, RenderTexture, Texture},
};

// -- Draw API
#[inline]
pub fn create_sprite<'a>() -> SpriteBuilder<'a> {
    SpriteBuilder::new()
}

#[inline]
pub fn create_render_sprite<'a>() -> RenderSpriteBuilder<'a> {
    RenderSpriteBuilder::new()
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
    Draw2D::with_target_extent(window_size(), gfx::frame_size())
}

#[inline]
pub fn create_draw_2d_for(target: &RenderTexture) -> Draw2D {
    Draw2D::with_target_extent(target.size(), target.size().as_uvec2())
}

#[inline]
pub(crate) fn clean_2d() {
    get_mut_2d_painter().clean();
}

// -- text
pub struct FontBuilder<'a> {
    source: &'a [u8],
    nearest: bool,
    line_height_pem: Option<f32>,
}

impl<'a> FontBuilder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            source: data,
            nearest: false,
            line_height_pem: None,
        }
    }

    pub fn with_nearest_filter(mut self, nearest: bool) -> Self {
        self.nearest = nearest;
        self
    }

    pub fn with_line_height_pem(mut self, pem: f32) -> Self {
        self.line_height_pem = Some(pem);
        self
    }

    // TODO from_system("Arial") it uses a system font (not supported on wasm)

    pub fn build(self) -> Result<Font, String> {
        let Self {
            source,
            nearest,
            line_height_pem,
        } = self;
        get_mut_text_system().create_font(source, nearest, line_height_pem)
    }
}

#[inline]
pub fn create_font(data: &[u8]) -> FontBuilder<'_> {
    FontBuilder::new(data)
}

#[inline]
pub fn set_default_font(font: &Font) {
    get_mut_text_system().set_default_font(font);
}

#[inline]
pub fn text_mask_atlas() -> Texture {
    get_text_system().mask.texture.texture().clone()
}

#[inline]
pub fn text_color_atlas() -> Texture {
    get_text_system().rgba_linear.texture.texture().clone()
}
