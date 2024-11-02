use crate::filters::sys::{IOFilterData, InOutTextures};
use crate::filters::Filter;
use crate::gfx;
use crate::gfx::{BindGroup, BindGroupLayout, BindingType, Buffer, RenderPipeline, Renderer};
use corelib::gfx::{Color, RenderTexture, TextureFilter};
use encase::{ShaderType, UniformBuffer};

// Based in the BlurFilter from pixi.js

// language=wgsl
const SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    {{STRUCT}}
}

@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_texture: sampler;

struct Blur {
    strength: f32,
    _pad: f32,
    _pad2: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> blur: Blur;

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

    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0)
    );

    let pos = positions[vertex_index];
    let final_uvs = uvs[vertex_index];

    let tex_size = vec2<f32>(textureDimensions(t_texture));
    let pixel_strength = blur.strength / tex_size.{{DIMENSION}};

    return VertexOutput(
       vec4<f32>(pos.x, pos.y * -1.0, 0.0, 1.0),
        {{VERTEX_OUT}}
    );
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var final_color = vec4<f32>(0.0);
    {{SAMPLING}}
    return final_color;
}
"#;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum KernelSize {
    #[default]
    Ks5 = 5,
    Ks7 = 7,
    Ks9 = 9,
    Ks11 = 11,
    Ks13 = 13,
    Ks15 = 15,
}

impl KernelSize {
    fn gaussian_values(&self) -> &[f32] {
        use KernelSize::*;
        match self {
            Ks5 => &[0.153388, 0.221461, 0.250301],
            Ks7 => &[0.071303, 0.131514, 0.189879, 0.214607],
            Ks9 => &[0.028532, 0.067234, 0.124009, 0.179044, 0.20236],
            Ks11 => &[0.0093, 0.028002, 0.065984, 0.121703, 0.175713, 0.198596],
            Ks13 => &[
                0.002406, 0.009255, 0.027867, 0.065666, 0.121117, 0.174868, 0.197641,
            ],
            Ks15 => &[
                0.000489, 0.002403, 0.009246, 0.02784, 0.065602, 0.120999, 0.174697, 0.197448,
            ],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, ShaderType)]
struct InnerBlurParams {
    #[align(16)]
    strength: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BlurParams {
    pub strength: f32,
    pub quality: f32,
    pub kernel_size: KernelSize,
}

impl Default for BlurParams {
    fn default() -> Self {
        Self {
            strength: 8.0,
            quality: 4.0,
            kernel_size: KernelSize::default(),
        }
    }
}

fn ubo_params(params: &BlurParams) -> InnerBlurParams {
    InnerBlurParams {
        strength: params.strength / params.quality,
    }
}

pub struct BlurFilter {
    pip_h: RenderPipeline,
    pip_v: RenderPipeline,

    ubo: Buffer,
    bind_group: BindGroup,
    ubs: UniformBuffer<[u8; 16]>,

    passes: usize,

    last_params: BlurParams,
    pub params: BlurParams,

    pub enabled: bool,
}

impl BlurFilter {
    pub fn new(params: BlurParams) -> Result<Self, String> {
        let pip_h = generate_shader(ShaderAxis::Horizontal, params.kernel_size)?;
        let pip_v = generate_shader(ShaderAxis::Vertical, params.kernel_size)?;

        // uniform buffer storage
        let mut ubs = UniformBuffer::new([0; 16]);
        ubs.write(&ubo_params(&params)).map_err(|e| e.to_string())?;

        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
            .with_label("BlurFilter UBO")
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_label("BlurFilter BindGroup(1)")
            .with_layout(pip_h.bind_group_layout_ref(1)?)
            .with_uniform(0, &ubo)
            .build()?;

        let passes = params.quality.round() as usize;

        Ok(Self {
            pip_h,
            pip_v,
            ubo,
            bind_group,
            ubs,
            passes,
            last_params: params,
            params,
            enabled: true,
        })
    }

    fn pass(
        &self,
        axis: ShaderAxis,
        output: &RenderTexture,
        bg_tex: &BindGroup,
    ) -> Result<(), String> {
        let pip = match axis {
            ShaderAxis::Horizontal => &self.pip_h,
            ShaderAxis::Vertical => &self.pip_v,
        };

        let mut renderer = Renderer::new();
        renderer
            .begin_pass()
            .pipeline(pip)
            .bindings(&[bg_tex, &self.bind_group])
            .draw(0..6);

        gfx::render_to_texture(output, &renderer)
    }
}

impl Filter for BlurFilter {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "BlurFilter"
    }

    fn apply(&self, data: IOFilterData) -> Result<bool, String> {
        if self.passes == 0 {
            return Ok(false);
        }

        if self.params.strength <= 0.0 {
            return Ok(false);
        }

        if self.passes == 1 {
            self.pass(ShaderAxis::Horizontal, data.temp.tex, data.input.bind_group)?;
            self.pass(ShaderAxis::Vertical, data.output.tex, data.temp.bind_group)?;
            return Ok(true);
        }

        let mut input = data.input;
        let mut output = data.temp;
        for _ in 0..self.passes - 1 {
            self.pass(ShaderAxis::Horizontal, output.tex, input.bind_group)?;
            std::mem::swap(&mut input, &mut output);
        }

        self.pass(ShaderAxis::Horizontal, data.output.tex, input.bind_group)?;

        let mut input = data.input;
        let mut output = data.temp;
        for _ in 0..self.passes - 1 {
            self.pass(ShaderAxis::Vertical, output.tex, input.bind_group)?;
            std::mem::swap(&mut input, &mut output);
        }

        self.pass(ShaderAxis::Vertical, data.output.tex, input.bind_group)?;

        Ok(true)
    }

    fn update(&mut self) -> Result<(), String> {
        if self.last_params != self.params {
            debug_assert_eq!(
                self.last_params.kernel_size, self.params.kernel_size,
                "Changing the KernelSize of BlurFilter after creation does nothing."
            );
            self.ubs
                .write(&ubo_params(&self.params))
                .map_err(|e| e.to_string())?;

            gfx::write_buffer(&self.ubo)
                .with_data(self.ubs.as_ref())
                .build()?;

            self.last_params = self.params;
            self.passes = self.params.quality.ceil() as usize;
        }

        Ok(())
    }

    fn texture_filter(&self) -> Option<TextureFilter> {
        Some(TextureFilter::Linear)
    }
}

#[derive(Copy, Clone, Debug)]
enum ShaderAxis {
    Horizontal,
    Vertical,
}

fn generate_shader(axis: ShaderAxis, ks: KernelSize) -> Result<RenderPipeline, String> {
    let kernel = ks.gaussian_values();
    let half_len = kernel.len() as f32;

    let mut blur_struct_src = String::from("");
    let mut blur_out_src = String::from("");
    let mut blur_sampling_src = String::from("");

    let ks = (ks as u8) as usize;
    for i in 0..ks {
        let n = i as f32;
        blur_struct_src.push_str(&format!("@location({n}) offset_{n}: vec2<f32>,\n    "));

        let v = n - half_len + 1.0;
        match axis {
            ShaderAxis::Horizontal => {
                blur_out_src.push_str(&format!(
                    "final_uvs + vec2({v} * pixel_strength, 0.0),\n    "
                ));
            }
            ShaderAxis::Vertical => {
                blur_out_src.push_str(&format!(
                    "final_uvs + vec2(0.0, {v} * pixel_strength),\n    "
                ));
            }
        }

        let kernel_index = if n < half_len { i } else { ks - i - 1 };
        let kernel_value = kernel[kernel_index];

        blur_sampling_src.push_str(&format!(
            "final_color += textureSample(t_texture, s_texture, in.offset_{n}) * {kernel_value};\n    "
        ));
    }

    let dimension = match axis {
        ShaderAxis::Horizontal => "x",
        ShaderAxis::Vertical => "y",
    };

    let shader = SHADER
        .replace("{{DIMENSION}}", dimension)
        .replace("{{STRUCT}}", &blur_struct_src)
        .replace("{{VERTEX_OUT}}", &blur_out_src)
        .replace("{{SAMPLING}}", &blur_sampling_src);

    gfx::create_render_pipeline(&shader)
        .with_label(&format!("BlurFilter {:?} Pipeline", axis))
        .with_bind_group_layout(
            BindGroupLayout::new()
                .with_entry(
                    BindingType::texture(0)
                        .with_fragment_visibility(true)
                        .with_vertex_visibility(true),
                )
                .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
        )
        .with_bind_group_layout(
            BindGroupLayout::new().with_entry(
                BindingType::uniform(0)
                    .with_fragment_visibility(true)
                    .with_vertex_visibility(true),
            ),
        )
        .build()
}
