mod alpha_fx;
mod blur_fx;
mod color_replace_fx;
mod gray_scale_fx;
mod pfx;
mod pixelate_fx;
mod rgb_split_fx;
mod sys;

use crate::gfx;
use crate::gfx::AsRenderer;
use sys::SYS;

pub use alpha_fx::*;
pub use blur_fx::*;
pub use color_replace_fx::*;
pub use gray_scale_fx::*;
pub use pfx::*;
pub use pixelate_fx::*;
pub use rgb_split_fx::*;
pub use sys::{IOPostFxData, InOutTextures};

#[inline]
pub fn render_to_pfx_frame<R>(renderer: &R) -> Result<(), String>
where
    R: AsRenderer,
{
    // the RT cloned to avoid borrow issues in case the user pass a PostProcess command
    // cloning a RT is cheap because all types inside are references or small numbers
    let rt = SYS.borrow_mut().check_and_get_pfx_frame()?.clone();
    gfx::render_to_texture(&rt, renderer)
}

#[inline]
pub fn present_pfx_frame(
    effects: &[&dyn PostFx],
    nearest_sampler: bool,
    clear_target: bool,
) -> Result<(), String> {
    SYS.borrow_mut()
        .present_pfx_frame(effects, nearest_sampler, clear_target)
}
