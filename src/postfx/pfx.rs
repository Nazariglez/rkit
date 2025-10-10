use crate::postfx::sys::{IOPostFxData, SYS};
use corelib::gfx::{
    self, AsRenderer, BindGroupLayout, BindingType, RenderPipeline, RenderPipelineBuilder,
    RenderTexture, TextureFilter,
};

pub trait PostFx {
    fn is_enabled(&self) -> bool;
    fn name(&self) -> &str;
    fn apply(&self, data: IOPostFxData) -> Result<bool, String>;
    fn update(&mut self) -> Result<(), String>;
    fn texture_filter(&self) -> Option<TextureFilter> {
        None
    }
}

pub struct PostProcess<'a, R>
where
    R: AsRenderer,
{
    pub effects: &'a [&'a dyn PostFx],
    pub render: &'a R,
    pub nearest_sampler: bool,
    pub clear_target: bool,
}

impl<'a, R> PostProcess<'a, R>
where
    R: AsRenderer,
{
    pub fn new(render: &'a R, effects: &'a [&'a dyn PostFx]) -> Self {
        Self {
            effects,
            render,
            nearest_sampler: false,
            clear_target: true,
        }
    }

    pub fn nearest_sampler(mut self, nearest: bool) -> Self {
        self.nearest_sampler = nearest;
        self
    }

    pub fn clear_target(mut self, clear: bool) -> Self {
        self.clear_target = clear;
        self
    }
}

impl<R> AsRenderer for PostProcess<'_, R>
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
    var positions = array<vec2<f32>, 6>(
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0,  1.0)
    );

    let pos = positions[vertex_index];
    let uvs = (pos + vec2<f32>(1.0, 1.0)) * 0.5;
    return VertexOutput(
        vec4<f32>(pos.x, pos.y * -1.0, 0.0, 1.0),
        uvs
    );
}

@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_texture: sampler;

";

pub fn create_pfx_pipeline<F: FnOnce(RenderPipelineBuilder) -> Result<RenderPipeline, String>>(
    fragment: &str,
    cb: F,
) -> Result<RenderPipeline, String> {
    let shader = format!("{VERT}\n{fragment}");
    let builder = gfx::create_render_pipeline(&shader)
        .with_label("PostFX RenderPipeline")
        .with_bind_group_layout(
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
