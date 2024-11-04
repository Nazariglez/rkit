mod blur;
mod color_replace;
mod gray_scale;
mod pixelate;
mod rgb_split;
mod sys;

use crate::filters::sys::{IOFilterData, SYS};
use crate::gfx;
use crate::gfx::{
    AsRenderer, BindGroupLayout, BindingType, RenderPipeline, RenderPipelineBuilder, RenderTexture,
    TextureFilter,
};

pub use blur::*;
pub use color_replace::*;
pub use gray_scale::*;
pub use pixelate::*;
pub use rgb_split::*;

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
pub fn present_pfx_frame(filters: &[&dyn Filter], nearest_sampler: bool) -> Result<(), String> {
    SYS.borrow_mut().present_pfx_frame(filters, nearest_sampler)
}

pub trait Filter {
    fn is_enabled(&self) -> bool;
    fn name(&self) -> &str;
    fn apply(&self, data: IOFilterData) -> Result<bool, String>;
    fn update(&mut self) -> Result<(), String>;
    fn texture_filter(&self) -> Option<TextureFilter> {
        None
    }
}

pub struct PostProcess<'a, R>
where
    R: AsRenderer,
{
    pub filters: &'a [&'a dyn Filter],
    pub render: &'a R,
    pub nearest_sampler: bool,
}

impl<'a, R> AsRenderer for PostProcess<'a, R>
where
    R: AsRenderer,
{
    fn render(&self, target: Option<&RenderTexture>) -> Result<(), String> {
        let mut sys = SYS.borrow_mut();
        sys.process(self, false, target)
    }
}

// language=wgsl
const VERT: &str = r"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uvs: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    var positions = array<vec2<f32>, 6>(
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0,  1.0)
    );

    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0)
    );

    // Access positions and UVs based on the vertex index
    let pos = positions[vertex_index];
    out.position = vec4<f32>(pos.x, pos.y * -1.0, 0.0, 1.0);
    out.uvs = uvs[vertex_index];
    return out;
}

@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_texture: sampler;

";

pub fn create_filter_pipeline<
    F: FnOnce(RenderPipelineBuilder) -> Result<RenderPipeline, String>,
>(
    fragment: &str,
    cb: F,
) -> Result<RenderPipeline, String> {
    let shader = format!("{}\n{}", VERT, fragment);
    let builder = gfx::create_render_pipeline(&shader).with_bind_group_layout(
        BindGroupLayout::new()
            .with_entry(
                BindingType::texture(0)
                    .with_fragment_visibility(true)
                    .with_vertex_visibility(true),
            )
            .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
    );
    cb(builder)
}
