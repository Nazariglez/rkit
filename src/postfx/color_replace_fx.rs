use crate::gfx;
use crate::gfx::{
    BindGroup, BindGroupLayout, BindingType, Buffer, Color, RenderPipeline, Renderer,
};
use crate::postfx::pfx::{PostFx, create_pfx_pipeline};
use crate::postfx::sys::IOPostFxData;
use encase::{ShaderType, UniformBuffer};

// language=wgsl
const FRAG: &str = r#"
struct ColorReplace {
    in_color: vec4<f32>,
    out_color: vec4<f32>,
    tolerance: f32,
};

@group(1) @binding(0)
var<uniform> color_replace: ColorReplace;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_texture, s_texture, in.uvs);
    let diff = distance(color.rgb, color_replace.in_color.rgb);

    if diff <= color_replace.tolerance {
        return color_replace.out_color;
    } else {
        return color;
    }
}
"#;

// TODO encase
#[derive(Copy, Clone, Debug, PartialEq, ShaderType)]
pub struct ColorReplaceParams {
    pub in_color: Color,
    pub out_color: Color,
    #[align(16)]
    pub tolerance: f32,
}

impl Default for ColorReplaceParams {
    fn default() -> Self {
        Self {
            in_color: Color::RED,
            out_color: Color::BLACK,
            tolerance: 0.4,
        }
    }
}

pub struct ColorReplaceFx {
    pip: RenderPipeline,
    ubo: Buffer,
    bind_group: BindGroup,
    ubs: UniformBuffer<[u8; 48]>,

    last_params: ColorReplaceParams,
    pub params: ColorReplaceParams,

    pub enabled: bool,
}

impl ColorReplaceFx {
    pub fn new(params: ColorReplaceParams) -> Result<Self, String> {
        let pip = create_pfx_pipeline(FRAG, |builder| {
            builder
                .with_label("ColorReplaceFx Pipeline")
                // this is bind group 1
                .with_bind_group_layout(
                    BindGroupLayout::new()
                        .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
                )
                .build()
        })?;

        let mut ubs = UniformBuffer::new([0; 48]);
        ubs.write(&params).map_err(|e| e.to_string())?;

        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
            .with_label("ColorReplaceFx UBO")
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_label("ColorReplaceFx BindGroup(1)")
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

impl PostFx for ColorReplaceFx {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "ColorReplaceFx"
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
