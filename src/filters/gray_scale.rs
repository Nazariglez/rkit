use crate::filters::{create_filter_pipeline, Filter};
use crate::gfx;
use crate::gfx::{BindGroup, BindGroupLayout, BindingType, Buffer, RenderPipeline};

// language=wgsl
const FRAG: &str = r#"
struct GrayScale {
    factor: f32,
}

@group(1) @binding(0)
var<uniform> gray_scale: GrayScale;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_texture, s_texture, in.uvs);
    let luminance = color.r * 0.3 + color.g * 0.59 + color.b * 0.11;
    let gray_color = vec4<f32>(vec3<f32>(luminance), color.a);
    return mix(color, gray_color, gray_scale.factor);
}"#;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GrayScaleParams {
    pub factor: f32,
}

impl Default for GrayScaleParams {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}

pub struct GrayScaleFilter {
    pip: RenderPipeline,
    ubo: Buffer,
    bind_groups: [BindGroup; 1],

    last_params: GrayScaleParams,
    pub params: GrayScaleParams,

    pub enabled: bool,
}

impl GrayScaleFilter {
    pub fn new(params: GrayScaleParams) -> Result<Self, String> {
        let pip = create_filter_pipeline(FRAG, |builder| {
            builder
                .with_label("GrayScaleFilter Pipeline")
                .with_bind_group_layout(
                    BindGroupLayout::default()
                        .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
                )
                .build()
        })?;

        let ubo = gfx::create_uniform_buffer(&[params.factor])
            .with_label("GrayScaleFilter UBO")
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_label("GrayScaleFilter BindGroup(1)")
            .with_layout(pip.bind_group_layout_ref(1)?)
            .with_uniform(0, &ubo)
            .build()?;

        Ok(Self {
            pip,
            ubo,
            bind_groups: [bind_group],
            last_params: params,
            params,
            enabled: true,
        })
    }
}

impl Filter for GrayScaleFilter {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn update(&mut self) -> Result<(), String> {
        if self.last_params != self.params {
            gfx::write_buffer(&self.ubo)
                .with_data(&[self.params.factor])
                .build()?;
            self.last_params = self.params;
        }

        Ok(())
    }

    fn pipeline(&self) -> &RenderPipeline {
        &self.pip
    }

    fn bind_groups(&self) -> &[BindGroup] {
        &self.bind_groups
    }
}
