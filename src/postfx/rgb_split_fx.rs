use crate::{
    gfx::{self, BindGroup, BindGroupLayout, BindingType, Buffer, RenderPipeline, Renderer},
    math::{Vec2, vec2},
    postfx::{
        pfx::{PostFx, create_pfx_pipeline},
        sys::IOPostFxData,
    },
};
use encase::{ShaderType, UniformBuffer};

// language=wgsl
const FRAG: &str = r#"
struct RgbSplit {
    red: vec2<f32>,
    green: vec2<f32>,
    blue: vec2<f32>,
    _pad: vec2<f32>
}

@group(1) @binding(0)
var<uniform> rgb_split: RgbSplit;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(t_texture));

    // Convert the pixel offsets into normalized UV space
    let red_uv_offset = rgb_split.red / tex_size;
    let green_uv_offset = rgb_split.green / tex_size;
    let blue_uv_offset = rgb_split.blue / tex_size;

    // Apply the normalized UV offsets to the original UV coordinates
    let red_uv = in.uvs + red_uv_offset;
    let green_uv = in.uvs + green_uv_offset;
    let blue_uv = in.uvs + blue_uv_offset;

    // Sample the texture with the respective offsets for each channel
    let s_red = textureSample(t_texture, s_texture, red_uv);
    let s_green = textureSample(t_texture, s_texture, green_uv);
    let s_blue = textureSample(t_texture, s_texture, blue_uv);

    // get colors and calculate alpha
    let red = s_red.r;
    let green = s_green.g;
    let blue = s_blue.b;
    let alpha = max(s_red.a, max(s_green.a, s_blue.a));

    // Combine the sampled channels into a single color
    return vec4<f32>(red, green, blue, alpha);
}"#;

#[derive(Copy, Clone, Debug, PartialEq, ShaderType)]
pub struct RgbSplitParams {
    pub red: Vec2,
    pub green: Vec2,
    #[shader(align(16))]
    pub blue: Vec2,
}

impl Default for RgbSplitParams {
    fn default() -> Self {
        Self {
            red: vec2(-10.0, 0.0),
            green: vec2(0.0, 10.0),
            blue: Vec2::ZERO,
        }
    }
}

pub struct RgbSplitFx {
    pip: RenderPipeline,
    ubo: Buffer,
    bind_group: BindGroup,
    ubs: UniformBuffer<[u8; 32]>,

    last_params: RgbSplitParams,
    pub params: RgbSplitParams,

    pub enabled: bool,
}

impl RgbSplitFx {
    pub fn new(params: RgbSplitParams) -> Result<Self, String> {
        let pip = create_pfx_pipeline(FRAG, |builder| {
            builder
                .with_label("RgbSplitFx Pipeline")
                .with_bind_group_layout(
                    BindGroupLayout::default()
                        .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
                )
                .build()
        })?;

        // uniform buffer storage
        let mut ubs = UniformBuffer::new([0; 32]);
        ubs.write(&params).map_err(|e| e.to_string())?;

        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
            .with_label("RgbSplitFx UBO")
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_label("RgbSplitFx BindGroup(1)")
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

impl PostFx for RgbSplitFx {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "RgbSplitFx"
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
