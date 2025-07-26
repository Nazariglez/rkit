use crate::backend::GfxBackendImpl;
use crate::backend::wgpu::frame::DrawFrame;
use crate::gfx::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingType, BlendMode,
    Buffer, BufferDescriptor, BufferUsage, GfxBackend, IndexFormat, RenderPipeline,
    RenderPipelineDescriptor, RenderTexture, RenderTextureDescriptor, Renderer, Sampler,
    SamplerDescriptor, TextureFilter, TextureFormat, VertexFormat, VertexLayout,
};

// TODO bufferless vertex?

// language=wgsl
const VERT: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position.x, model.position.y * -1.0, 0.0, 1.0);
    return out;
}
"#;

// language=wgsl
const FRAG: &str = r#"
@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_texture: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_texture, s_texture, in.tex_coords);
}
"#;

// language=wgsl
const FRAG_TO_SRGB: &str = r#"
@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_texture: sampler;

/// Converts a single channel from linear to gamma (sRGB) space
fn linear_to_gamma(x: f32) -> f32 {
    let a: f32 = 0.055;
    return select(12.92 * x, (1.0 + a) * pow(x, 1.0 / 2.4) - a, x > 0.0031308);
}

/// Converts an entire color from linear to sRGB space
fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_to_gamma(color.r),
        linear_to_gamma(color.g),
        linear_to_gamma(color.b)
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let linear_color = textureSample(t_texture, s_texture, in.tex_coords);
    let srgb_color = linear_to_srgb(linear_color.rgb);
    return vec4<f32>(srgb_color, linear_color.a);
}
"#;

// Compatibility layer between Web and Native surfaces
pub(crate) struct OffscreenSurfaceData {
    pub texture: RenderTexture,
    pub sampler: Sampler,
    pub pip: RenderPipeline,
    pub vbo: Buffer,
    pub ebo: Buffer,
    pub bind_group: BindGroup,
}

impl OffscreenSurfaceData {
    pub fn new(gfx: &mut GfxBackend, pixelated: bool) -> Result<Self, String> {
        let texture = gfx.create_render_texture(RenderTextureDescriptor {
            label: Some("Offscreen Surface"),
            depth: false,
            width: gfx.surface.config.width,
            height: gfx.surface.config.height,
            format: None,
        })?;

        let filter = if pixelated {
            TextureFilter::Nearest
        } else {
            TextureFilter::Linear
        };

        let sampler = gfx.create_sampler(SamplerDescriptor {
            label: Some("Offscreen Surface Sampler"),
            mag_filter: filter,
            min_filter: filter,
            ..Default::default()
        })?;

        // output always srgb values even if the device does not support a srgb surface
        // is this the right thing to do?
        let (vert, frag) = if gfx.surface.raw_format.is_srgb() {
            (VERT, FRAG)
        } else {
            (VERT, FRAG_TO_SRGB)
        };

        let pip = gfx.create_render_pipeline(RenderPipelineDescriptor {
            label: Some("Offscreen Surface Pipeline"),
            shader: &format!("{vert}\n{frag}"),
            vertex_layout: (&[VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x2)] as &[_])
                .try_into()
                .unwrap(),
            blend_mode: Some(BlendMode::NORMAL),
            index_format: IndexFormat::UInt16,
            compatible_textures: (&[TextureFormat::from_wgpu(gfx.surface.raw_format).unwrap()]
                as &[_])
                .try_into()
                .unwrap(),
            bind_group_layout: (&[BindGroupLayout::new()
                .with_entry(BindingType::texture(0).with_fragment_visibility(true))
                .with_entry(BindingType::sampler(1).with_fragment_visibility(true))]
                as &[_])
                .try_into()
                .unwrap(),
            ..Default::default()
        })?;

        #[rustfmt::skip]
        let vertices: &[f32] = &[
            1.0,  1.0,      1.0, 1.0,
            1.0, -1.0,      1.0, 0.0,
            -1.0, -1.0,     0.0, 0.0,
            -1.0,  1.0,     0.0, 1.0,
        ];

        let vbo = gfx.create_buffer(BufferDescriptor {
            label: Some("Offscreen Surface VBO"),
            usage: BufferUsage::Vertex,
            content: bytemuck::cast_slice(vertices),
            write: false,
        })?;

        #[rustfmt::skip]
        let indices: &[u16] = &[
            0, 1, 3,
            1, 2, 3,
        ];

        let ebo = gfx.create_buffer(BufferDescriptor {
            label: Some("Offscreen Surface EBO"),
            usage: BufferUsage::Index,
            content: bytemuck::cast_slice(indices),
            write: false,
        })?;

        let bind_group = gfx.create_bind_group(BindGroupDescriptor {
            label: Some("Offscreen Surface BindGroup"),
            layout: Some(pip.bind_group_layout_ref(0)?),
            entry: (&[
                BindGroupEntry::Texture {
                    location: 0,
                    texture: &texture,
                },
                BindGroupEntry::Sampler {
                    location: 1,
                    sampler: &sampler,
                },
            ] as &[_])
                .try_into()
                .unwrap(),
        })?;

        Ok(Self {
            texture,
            sampler,
            pip,
            vbo,
            ebo,
            bind_group,
        })
    }

    pub fn update(&mut self, gfx: &mut GfxBackend) -> Result<(), String> {
        let same_width = gfx.surface.config.width == self.texture.width() as u32;
        let same_height = gfx.surface.config.height == self.texture.height() as u32;
        let needs_update = !(same_width && same_height);
        if !needs_update {
            // do nothing
            return Ok(());
        }

        let texture = gfx.create_render_texture(RenderTextureDescriptor {
            label: Some("Offscreen Surface"),
            depth: false,
            width: gfx.surface.config.width,
            height: gfx.surface.config.height,
            format: None,
        })?;
        self.texture = texture;

        self.bind_group = gfx.create_bind_group(BindGroupDescriptor {
            label: Some("Offscreen Surface BindGroup"),
            layout: Some(self.pip.bind_group_layout_ref(0)?),
            entry: (&[
                BindGroupEntry::Texture {
                    location: 0,
                    texture: &self.texture,
                },
                BindGroupEntry::Sampler {
                    location: 1,
                    sampler: &self.sampler,
                },
            ] as &[_])
                .try_into()
                .unwrap(),
        })?;

        Ok(())
    }

    pub fn present(&self, gfx: &mut GfxBackend, frame: &mut DrawFrame) -> Result<(), String> {
        let mut renderer = Renderer::new();
        renderer
            .begin_pass()
            .pipeline(&self.pip)
            .buffers(&[&self.vbo, &self.ebo])
            .bindings(&[&self.bind_group])
            .draw(0..6);

        gfx.render_to_frame(frame, &renderer)
    }
}
