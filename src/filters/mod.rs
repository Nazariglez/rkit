mod color_replace;
mod gray_scale;
mod pixelate;
mod sys;

use crate::filters::sys::{InOutTextures, SYS};
use crate::gfx;
use crate::gfx::{
    AsRenderer, BindGroup, BindGroupLayout, BindingType, RenderPipeline, RenderPipelineBuilder,
    RenderTexture, TextureFilter,
};

pub use color_replace::*;
pub use gray_scale::*;
pub use pixelate::*;
// pub use blur::*;

pub trait Filter {
    fn is_enabled(&self) -> bool;
    fn name(&self) -> &str;
    fn apply(&self, io_tex: &mut InOutTextures, bg_tex: &BindGroup) -> Result<(), String>;
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
    pub pixelated: bool,
}

impl<'a, R> AsRenderer for PostProcess<'a, R>
where
    R: AsRenderer,
{
    fn render(&self, target: Option<&RenderTexture>) -> Result<(), String> {
        let mut sys = SYS.borrow_mut();
        sys.process(self, target)
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
            .with_entry(BindingType::texture(0).with_fragment_visibility(true))
            .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
    );
    cb(builder)
}
