use crate::{
    gfx::{self, BindGroup, BindGroupLayout, BindingType, Buffer, Color, RenderPipeline, Renderer},
    math::{Vec2, vec2},
    postfx::{IOPostFxData, PostFx, create_pfx_pipeline},
};
use encase::{ShaderType, UniformBuffer};

// language=wgsl
const FRAG: &str = r#"
struct Shadow {
    color: vec3f,
    offset: vec2f,
    alpha: f32,
}

@group(1) @binding(0)
var<uniform> shadow: Shadow;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let size = vec2f(textureDimensions(t_texture));
    let offset = shadow.offset / size;
    let color_offset = textureSample(t_texture, s_texture, in.uvs - offset);
    let shadow_color = vec4f(shadow.color.rgb * color_offset.a, color_offset.a * shadow.alpha);
    let color = textureSample(t_texture, s_texture, in.uvs);
    return mix(shadow_color, color, color.a);
}"#;

#[derive(Copy, Clone, Debug, PartialEq, ShaderType)]
pub struct ShadowParams {
    pub color: Color,
    pub offset: Vec2,
    pub alpha: f32,
}

impl Default for ShadowParams {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            offset: vec2(9.0, 9.0),
            alpha: 0.4,
        }
    }
}

pub struct ShadowFx {
    pip: RenderPipeline,
    ubo: Buffer,
    bind_group: BindGroup,
    ubs: UniformBuffer<Vec<u8>>,

    last_params: ShadowParams,
    pub params: ShadowParams,

    pub enabled: bool,
}

impl ShadowFx {
    pub fn new(params: ShadowParams) -> Result<Self, String> {
        let pip = create_pfx_pipeline(FRAG, |builder| {
            builder
                .with_label("ShadowFx Pipeline")
                .with_bind_group_layout(
                    BindGroupLayout::default()
                        .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
                )
                .build()
        })?;

        let mut ubs = UniformBuffer::new(vec![]);
        ubs.write(&params).map_err(|e| e.to_string())?;

        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
            .with_label("ShadowFx UBO")
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_label("ShadowFx BindGroup(1)")
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

impl PostFx for ShadowFx {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "ShadowFx"
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
