use crate::filters::sys::IOFilterData;
use crate::filters::{create_filter_pipeline, Filter};
use crate::gfx;
use crate::gfx::{BindGroup, BindGroupLayout, BindingType, Buffer, RenderPipeline, Renderer};
use crate::math::Vec2;
use corelib::gfx::TextureFilter;
use encase::{ShaderType, UniformBuffer};

// language=wgsl
const FRAG: &str = r#"
struct PixelData {
    size: vec2<f32>,
    _pad: vec2<f32>
};

@group(1) @binding(0)
var<uniform> pixel_data: PixelData;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(t_texture));
    let shifted_uvs = in.uvs;
    let coords = floor((shifted_uvs * tex_size) / pixel_data.size) * pixel_data.size;
    let uvs = (coords / tex_size);
    return textureSample(t_texture, s_texture, uvs);
}
"#;

#[derive(Copy, Clone, Debug, PartialEq, ShaderType)]
pub struct PixelateParams {
    #[align(16)]
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
    ubs: UniformBuffer<[u8; 16]>,

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

        // uniform buffer storage
        let mut ubs = UniformBuffer::new([0; 16]);
        ubs.write(&params).map_err(|e| e.to_string())?;

        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
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
            ubs,
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

    fn apply(&self, data: IOFilterData) -> Result<bool, String> {
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

    fn texture_filter(&self) -> Option<TextureFilter> {
        Some(TextureFilter::Nearest)
    }
}
