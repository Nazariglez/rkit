use crate::filters::{create_filter_pipeline, Filter};
use crate::gfx;
use crate::gfx::{BindGroup, BindGroupLayout, BindingType, Buffer, RenderPipeline};
use crate::math::Vec2;

// language=wgsl
const FRAG: &str = r#"
struct PixelData {
    size: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> pixel_data: PixelData;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
let tex_size = vec2<f32>(textureDimensions(t_texture));
    let shifted_uvs = in.uvs - 0.5;
    let coords = floor((shifted_uvs * tex_size) / pixel_data.size) * pixel_data.size;
    let uvs = (coords / tex_size) + 0.5;
    return textureSample(t_texture, s_texture, uvs) * in.color;
}
"#;

pub struct PixelateFilter {
    pip: RenderPipeline,
    ubo: Buffer,
    bind_group: BindGroup,

    last_size: Vec2,
    pub size: Vec2,
}

impl PixelateFilter {
    pub fn new() -> Result<Self, String> {
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

        let size = Vec2::splat(10.0);

        let ubo = gfx::create_uniform_buffer(size.as_ref())
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
            last_size: size,
            size,
        })
    }
}

impl Filter for PixelateFilter {
    fn pipeline(&self) -> &RenderPipeline {
        &self.pip
    }
}
