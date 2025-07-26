use crate::{
    gfx::{self, BindGroup, BindGroupLayout, BindingType, Buffer, RenderPipeline, Renderer},
    postfx::{
        pfx::{PostFx, create_pfx_pipeline},
        sys::IOPostFxData,
    },
};
use encase::{ShaderType, UniformBuffer};

// language=wgsl
const FRAG: &str = r#"
struct GrayScale {
    factor: f32,
    _pad: f32,
    _pad2: vec2<f32>,
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

#[derive(Copy, Clone, Debug, PartialEq, ShaderType)]
pub struct GrayScaleParams {
    #[align(16)]
    pub factor: f32,
}

impl Default for GrayScaleParams {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}

pub struct GrayScaleFx {
    pip: RenderPipeline,
    ubo: Buffer,
    bind_group: BindGroup,
    ubs: UniformBuffer<[u8; 16]>,

    last_params: GrayScaleParams,
    pub params: GrayScaleParams,

    pub enabled: bool,
}

impl GrayScaleFx {
    pub fn new(params: GrayScaleParams) -> Result<Self, String> {
        let pip = create_pfx_pipeline(FRAG, |builder| {
            builder
                .with_label("GrayScaleFx Pipeline")
                .with_bind_group_layout(
                    BindGroupLayout::default()
                        .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
                )
                .build()
        })?;

        // uniform buffer storage
        let mut ubs = UniformBuffer::new([0; 16]);
        ubs.write(&params).map_err(|e| e.to_string())?;

        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
            .with_label("GrayScaleFx UBO")
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_label("GrayScaleFx BindGroup(1)")
            .with_layout(pip.bind_group_layout_ref(1)?)
            .with_uniform(0, &ubo)
            .build()?;

        Ok(Self {
            pip,
            ubo,
            bind_group,
            ubs,
            last_params: params,
            params,
            enabled: true,
        })
    }
}

impl PostFx for GrayScaleFx {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "GrayScaleFx"
    }

    fn apply(&self, data: IOPostFxData) -> Result<bool, String> {
        let mut renderer = Renderer::new();
        renderer
            .begin_pass()
            .pipeline(&self.pip)
            .bindings(&[data.input.bind_group, &self.bind_group])
            .draw(0..6);

        gfx::render_to_texture(data.output.tex, &renderer)?;
        Ok(true)
    }

    fn update(&mut self) -> Result<(), String> {
        if self.last_params != self.params {
            self.ubs.write(&self.params).map_err(|e| e.to_string())?;

            gfx::write_buffer(&self.ubo)
                .with_data(self.ubs.as_ref())
                .build()?;
            self.last_params = self.params;
        }

        Ok(())
    }
}
