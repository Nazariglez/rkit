use crate::app::window_size;
use crate::filters::{create_filter_pipeline, Filter, PostProcess};
use crate::gfx::{
    self, AsRenderer, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingType,
    BlendMode, Buffer, BufferDescriptor, BufferUsage, IndexFormat, RenderPipeline,
    RenderPipelineDescriptor, RenderTexture, RenderTextureDescriptor, RenderTextureId, Renderer,
    Sampler, SamplerDescriptor, SamplerId, TextureFilter, TextureFormat, VertexFormat,
    VertexLayout,
};
use crate::math::UVec2;
use arrayvec::ArrayVec;
use atomic_refcell::AtomicRefCell;
use gfx::consts::MAX_BIND_GROUPS_PER_PIPELINE;
use once_cell::sync::Lazy;
use std::num::NonZeroUsize;
use utils::fast_cache::FastCache;

const MAX_CACHED_TEXTURES: usize = 12;

pub(crate) static SYS: Lazy<AtomicRefCell<PostProcessSys>> =
    Lazy::new(|| AtomicRefCell::new(PostProcessSys::new().unwrap()));

// language=wgsl
const FRAG: &str = r#"
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_texture, s_texture, in.uvs);
}
"#;

#[derive(Hash, Copy, Clone, Eq, PartialEq)]
struct BindGroupKey {
    tex: RenderTextureId,
    sampler: SamplerId,
}

struct IOTextures {
    in_tex: RenderTexture,
    out_tex: RenderTexture,
}

impl IOTextures {
    fn new(size: UVec2) -> Result<Self, String> {
        let in_tex = gfx::create_render_texture()
            .with_label(&format!("PostProcess In Texture ({}, {})", size.x, size.y))
            .with_size(size.x, size.y)
            .build()?;

        let out_tex = gfx::create_render_texture()
            .with_label(&format!("PostProcess Out Texture ({}, {})", size.x, size.y))
            .with_size(size.x, size.y)
            .build()?;

        Ok(Self { in_tex, out_tex })
    }
}

pub(crate) struct PostProcessSys {
    textures: FastCache<UVec2, IOTextures>,
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

        let pip = create_filter_pipeline(FRAG, |builder| {
            builder
                .with_label("PostProcess pipeline")
                .with_blend_mode(BlendMode::NORMAL)
                .build()
        })?;

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
        // skip process if there is no filters
        if info.filters.is_empty() {
            return match target {
                None => gfx::render_to_frame(info.render),
                Some(rt) => gfx::render_to_texture(rt, info.render),
            };
        }

        // filter
        let size = target
            .map(|rt| rt.size())
            .unwrap_or_else(|| window_size())
            .as_uvec2();

        let io_tex = self.textures.get_or_insert_mut(size, || {
            log::info!(
                "Creating PostProcess IOTextures with size: ({}, {})",
                size.x,
                size.y
            );
            IOTextures::new(size).unwrap() // TODO maybe this is better to raise the error somehow?
        });
        gfx::render_to_texture(&io_tex.in_tex, info.render)?;

        let sampler = if info.pixelated {
            &self.nearest_sampler
        } else {
            &self.linear_sampler
        };

        // filter pass
        info.filters
            .iter()
            .filter(|f| f.is_enabled())
            .for_each(|filter| {
                // clear any group form last filter
                let mut bind_groups: ArrayVec<&BindGroup, MAX_BIND_GROUPS_PER_PIPELINE> =
                    ArrayVec::new();

                let sampler = filter
                    .texture_filter()
                    .map(|tf| match tf {
                        TextureFilter::Linear => &self.linear_sampler,
                        TextureFilter::Nearest => &self.nearest_sampler,
                    })
                    .unwrap_or(sampler);

                // prepare a new bing_group if needed
                let bg_key = BindGroupKey {
                    tex: io_tex.in_tex.id(),
                    sampler: sampler.id(),
                };
                let bind_group = self.bind_groups.get_or_insert(bg_key, || {
                    gfx::create_bind_group()
                        .with_label("PostProcess BindGroup")
                        .with_layout(self.pip.bind_group_layout_ref(0).unwrap())
                        .with_texture(0, io_tex.in_tex.texture())
                        .with_sampler(1, sampler)
                        .build()
                        .unwrap() // TODO raise error...
                });

                // TODO custom sampler per filter?
                let f_pip = filter.pipeline();
                let f_bind_groups = filter.bind_groups();

                bind_groups.push(bind_group);
                f_bind_groups.iter().for_each(|bg| bind_groups.push(bg));

                let mut renderer = Renderer::new();
                renderer
                    .begin_pass()
                    .pipeline(f_pip)
                    .buffers(&[&self.vbo, &self.ebo])
                    .bindings(&bind_groups)
                    .draw(0..6);

                gfx::render_to_texture(&io_tex.out_tex, &renderer).unwrap();
                std::mem::swap(&mut io_tex.in_tex, &mut io_tex.out_tex);
            });

        // final pass
        let bg_key = BindGroupKey {
            tex: io_tex.in_tex.id(),
            sampler: sampler.id(),
        };
        let bind_group = self.bind_groups.get_or_insert(bg_key, || {
            gfx::create_bind_group()
                .with_label("PostProcess BindGroup")
                .with_layout(self.pip.bind_group_layout_ref(0).unwrap())
                .with_texture(0, io_tex.in_tex.texture())
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
