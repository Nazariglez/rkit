use crate::app::window_size;
use crate::filters::PostProcess;
use crate::gfx;
use crate::gfx::{
    AsRenderer, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingType,
    BlendMode, Buffer, BufferDescriptor, BufferUsage, IndexFormat, RenderPipeline,
    RenderPipelineDescriptor, RenderTexture, RenderTextureDescriptor, RenderTextureId, Renderer,
    Sampler, SamplerDescriptor, SamplerId, TextureFilter, TextureFormat, VertexFormat,
    VertexLayout,
};
use crate::math::UVec2;
use atomic_refcell::AtomicRefCell;
use once_cell::sync::Lazy;
use std::num::NonZeroUsize;
use utils::fast_cache::FastCache;

const MAX_CACHED_TEXTURES: usize = 12;

pub(crate) static SYS: Lazy<AtomicRefCell<PostProcessSys>> =
    Lazy::new(|| AtomicRefCell::new(PostProcessSys::new().unwrap()));

// language=wgsl
const SHADER: &str = r#"
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

@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_texture: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_texture, s_texture, in.tex_coords);
}
"#;

#[derive(Hash, Copy, Clone, Eq, PartialEq)]
struct BindGroupKey {
    tex: RenderTextureId,
    sampler: SamplerId,
}

pub(crate) struct PostProcessSys {
    textures: FastCache<UVec2, RenderTexture>,
    bind_groups: FastCache<BindGroupKey, BindGroup>,
    linear_sampler: Sampler,
    nearest_sampler: Sampler,
    pip: RenderPipeline,
    vbo: Buffer,
    ebo: Buffer,
}

impl PostProcessSys {
    pub fn new() -> Result<Self, String> {
        let textures = FastCache::new(
            NonZeroUsize::new(MAX_CACHED_TEXTURES)
                .ok_or_else(|| "Max Cached Textures cannot be 0".to_string())?,
        );
        let bind_groups = FastCache::new(
            NonZeroUsize::new(10).ok_or_else(|| "BindGroup amount cannot be 0".to_string())?,
        );

        let linear_sampler = gfx::create_sampler()
            .with_label("PostProcess linear sampler")
            .with_min_filter(TextureFilter::Linear)
            .with_mag_filter(TextureFilter::Linear)
            .build()?;

        let nearest_sampler = gfx::create_sampler()
            .with_label("PostProcess nearest sampler")
            .with_min_filter(TextureFilter::Nearest)
            .with_mag_filter(TextureFilter::Nearest)
            .build()?;

        let pip = gfx::create_render_pipeline(SHADER)
            .with_label("PostProcess pipeline")
            .with_vertex_layout(
                VertexLayout::new()
                    .with_attr(0, VertexFormat::Float32x2)
                    .with_attr(1, VertexFormat::Float32x2),
            )
            .with_index_format(IndexFormat::UInt16)
            .with_bind_group_layout(
                BindGroupLayout::new()
                    .with_entry(BindingType::texture(0).with_fragment_visibility(true))
                    .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
            )
            .with_blend_mode(BlendMode::NORMAL)
            .build()?;

        #[rustfmt::skip]
        let vertices: &[f32] = &[
            1.0,  1.0,      1.0, 1.0,
            1.0, -1.0,      1.0, 0.0,
            -1.0, -1.0,     0.0, 0.0,
            -1.0,  1.0,     0.0, 1.0,

            1.0,  1.0,      1.0, 1.0,
            1.0, -1.0,      1.0, 0.0,
            -1.0, -1.0,     0.0, 0.0,
            -1.0,  1.0,     0.0, 1.0,
        ];

        let vbo = gfx::create_vertex_buffer(vertices)
            .with_label("PostProcess VBO")
            .build()?;

        #[rustfmt::skip]
        let indices: &[u16] = &[
            0, 1, 3,
            1, 2, 3,

            4, 5, 7,
            5, 6, 7,
        ];

        let ebo = gfx::create_index_buffer(indices)
            .with_label("PostProcess EBO")
            .build()?;

        Ok(Self {
            textures,
            bind_groups,
            linear_sampler,
            nearest_sampler,
            pip,
            vbo,
            ebo,
        })
    }

    pub fn process<R: AsRenderer>(
        &mut self,
        info: &PostProcess<R>,
        target: Option<&RenderTexture>,
    ) -> Result<(), String> {
        let size = target
            .map(|rt| rt.size())
            .unwrap_or_else(|| window_size())
            .as_uvec2();

        let rt = self.textures.get_or_insert(size, || {
            log::info!(
                "Creating PostProcess Texture with size: ({}, {})",
                size.x,
                size.y
            );
            gfx::create_render_texture()
                .with_label(&format!("PostProcess Texture ({}, {})", size.x, size.y))
                .with_size(size.x, size.y)
                .build()
                .unwrap() // TODO maybe this is better to raise the error somehow?
        });
        gfx::render_to_texture(rt, info.render)?;

        let sampler = if info.pixelated {
            &self.nearest_sampler
        } else {
            &self.linear_sampler
        };

        let bg_key = BindGroupKey {
            tex: rt.id(),
            sampler: sampler.id(),
        };
        let bind_group = self.bind_groups.get_or_insert(bg_key, || {
            gfx::create_bind_group()
                .with_label("PostProcess BindGroup")
                .with_layout(self.pip.bind_group_layout_ref(0).unwrap())
                .with_texture(0, rt.texture())
                .with_sampler(1, sampler)
                .build()
                .unwrap() // TODO raise error...
        });

        let mut renderer = Renderer::new();
        renderer
            .begin_pass()
            .pipeline(&self.pip)
            .buffers(&[&self.vbo, &self.ebo])
            .bindings(&[bind_group])
            .draw(0..6);

        match target {
            None => gfx::render_to_frame(&renderer),
            Some(t) => gfx::render_to_texture(t, &renderer),
        }
    }
}
