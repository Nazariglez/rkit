use crate::filters::sys::InOutTextures;
use crate::filters::{create_filter_pipeline, Filter};
use crate::gfx;
use crate::gfx::{BindGroup, BindGroupLayout, BindingType, Buffer, RenderPipeline, Renderer};
use crate::math::Vec2;

// language=wgsl
const FRAG: &str = r#"
struct PixelData {
    size: vec4<f32>, // it's a vec2 but webgl has alignment issues
};

@group(1) @binding(0)
var<uniform> pixel_data: PixelData;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
let tex_size = vec2<f32>(textureDimensions(t_texture));
    let shifted_uvs = in.uvs - 0.5;
    let coords = floor((shifted_uvs * tex_size) / pixel_data.size.xy) * pixel_data.size.xy;
    let uvs = (coords / tex_size) + 0.5;
    return textureSample(t_texture, s_texture, uvs);
}
"#;

// TODO encase
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PixelateParams {
    pub size: Vec2,
}

impl Default for PixelateParams {
    fn default() -> Self {
        Self {
            size: Vec2::splat(10.0),
        }
    }
}

pub struct PixelateFilter {
    pip: RenderPipeline,
    ubo: Buffer,
    bind_group: BindGroup,

    last_params: PixelateParams,
    pub params: PixelateParams,

    pub enabled: bool,
}

impl PixelateFilter {
    pub fn new(params: PixelateParams) -> Result<Self, String> {
        let pip = create_filter_pipeline(FRAG, |builder| {
            builder
                .with_label("PixelateFilter Pipeline")
                // this is bind group 1
                .with_bind_group_layout(
                    BindGroupLayout::new()
                        .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
                )
                .build()
        })?;

        let ubo = gfx::create_uniform_buffer(&[params.size.x, params.size.y, 0.0, 0.0])
            .with_label("PixelateFilter UBO")
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_label("PixelateFilter BindGroup(1)")
            .with_layout(pip.bind_group_layout_ref(1)?)
            .with_uniform(0, &ubo)
            .build()?;

        Ok(Self {
            pip,
            ubo,
            bind_group,
            last_params: params,
            params,
            enabled: true,
        })
    }
}

impl Filter for PixelateFilter {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "PixelateFilter"
    }

    fn apply(&self, io_tex: &mut InOutTextures, bg_tex: &BindGroup) -> Result<(), String> {
        let mut renderer = Renderer::new();
        renderer
            .begin_pass()
            .pipeline(&self.pip)
            .bindings(&[bg_tex, &self.bind_group])
            .draw(0..6);

        gfx::render_to_texture(io_tex.output(), &renderer)
    }

    fn update(&mut self) -> Result<(), String> {
        if self.last_params != self.params {
            gfx::write_buffer(&self.ubo)
                .with_data(&[self.params.size.x, self.params.size.y, 0.0, 0.0])
                .build()?;
            self.last_params = self.params;
        }

        Ok(())
    }
}
