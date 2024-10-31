mod blur;
mod color_replace;
mod gray_scale;
mod pixelate;
mod sys;

use crate::filters::sys::SYS;
use crate::gfx;
use crate::gfx::{
    AsRenderer, BindGroup, BindGroupLayout, BindingType, IndexFormat, RenderPipeline,
    RenderPipelineBuilder, RenderTexture, TextureFilter, VertexFormat, VertexLayout,
};

pub use color_replace::*;
pub use gray_scale::*;
pub use pixelate::*;
// pub use blur::*;

pub trait Filter {
    // TODO use "apply" instead to allow multipass filters?
    fn is_enabled(&self) -> bool;
    fn update(&mut self) -> Result<(), String>;
    fn pipeline(&self) -> &RenderPipeline;
    fn bind_groups(&self) -> &[BindGroup];
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
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uvs: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uvs: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.uvs = model.uvs;
    out.position = vec4<f32>(model.position.x, model.position.y * -1.0, 0.0, 1.0);
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
    let builder = gfx::create_render_pipeline(&shader)
        .with_vertex_layout(
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x2),
        )
        .with_index_format(IndexFormat::UInt16)
        .with_bind_group_layout(
            BindGroupLayout::new()
                .with_entry(BindingType::texture(0).with_fragment_visibility(true))
                .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
        );
    cb(builder)
}
