use crate::app::window_size;
use crate::gfx::{
    self, AsRenderer, BindGroup, BlendMode, RenderPipeline, RenderTexture, RenderTextureId,
    Renderer, Sampler, SamplerId, TextureFilter,
};
use crate::math::UVec2;
use crate::postfx::pfx::{create_pfx_pipeline, PostFx, PostProcess};
use atomic_refcell::AtomicRefCell;
use corelib::gfx::Color;
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

pub struct InOutTextures {
    pub in_rt: RenderTexture,
    pub out_rt: RenderTexture,
    pub temp_rt: RenderTexture,
}

impl InOutTextures {
    fn new(size: UVec2) -> Result<Self, String> {
        let in_tex = gfx::create_render_texture()
            .with_label(&format!("PostProcess Texture 1 - ({}, {})", size.x, size.y))
            .with_size(size.x, size.y)
            .build()?;

        let out_tex = gfx::create_render_texture()
            .with_label(&format!("PostProcess Texture 2 - ({}, {})", size.x, size.y))
            .with_size(size.x, size.y)
            .build()?;

        let temp_tex = gfx::create_render_texture()
            .with_label(&format!(
                "PostProcess Texture Temp - ({}, {})",
                size.x, size.y
            ))
            .with_size(size.x, size.y)
            .build()?;

        Ok(Self {
            in_rt: in_tex,
            out_rt: out_tex,
            temp_rt: temp_tex,
        })
    }

    pub fn swap(&mut self) {
        std::mem::swap(&mut self.in_rt, &mut self.out_rt);
    }
}

#[derive(Copy, Clone)]
pub struct TextureBindGroup<'a> {
    pub tex: &'a RenderTexture,
    pub bind_group: &'a BindGroup,
}

#[derive(Copy, Clone)]
pub struct IOPostFxData<'a> {
    pub input: TextureBindGroup<'a>,
    pub output: TextureBindGroup<'a>,
    pub temp: TextureBindGroup<'a>,
}

pub(crate) struct PostProcessSys {
    frame_rt: Option<RenderTexture>,
    textures: FastCache<UVec2, InOutTextures>,
    bind_groups: FastCache<BindGroupKey, BindGroup>,
    linear_sampler: Sampler,
    nearest_sampler: Sampler,
    pip: RenderPipeline,
}

macro_rules! insert_bg {
    ($self:ident, $rt:expr, $sampler:expr) => {{
        let bg_key = BindGroupKey {
            tex: $rt.id(),
            sampler: $sampler.id(),
        };

        if !$self.bind_groups.contains_key(&bg_key) {
            $self.bind_groups.insert(
                bg_key,
                gfx::create_bind_group()
                    .with_label("PostProcess BindGroup")
                    .with_layout($self.pip.bind_group_layout_ref(0).unwrap())
                    .with_texture(0, $rt)
                    .with_sampler(1, $sampler)
                    .build()
                    .unwrap(),
            );
        }

        $self.bind_groups.promote(&bg_key);
    }};
}

macro_rules! get_bg {
    ($self:ident, $rt:expr, $sampler:expr) => {{
        let bg_key = BindGroupKey {
            tex: $rt.id(),
            sampler: $sampler.id(),
        };

        $self.bind_groups.peek(&bg_key).unwrap()
    }};
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

        let pip = create_pfx_pipeline(FRAG, |builder| {
            builder
                .with_label("PostProcess pipeline")
                .with_blend_mode(BlendMode::NORMAL)
                .build()
        })?;

        Ok(Self {
            frame_rt: None,
            textures,
            bind_groups,
            linear_sampler,
            nearest_sampler,
            pip,
        })
    }

    pub fn process<R: AsRenderer>(
        &mut self,
        info: &PostProcess<R>,
        is_presenting_frame: bool,
        target: Option<&RenderTexture>,
    ) -> Result<(), String> {
        // skip process if there is no effects
        let is_empty = info.effects.is_empty();
        let all_disabled = info.effects.iter().find(|fx| fx.is_enabled()).is_none();
        if !is_presenting_frame && (is_empty || all_disabled) {
            return match target {
                None => gfx::render_to_frame(info.render),
                Some(rt) => gfx::render_to_texture(rt, info.render),
            };
        }

        // effect
        let size = target
            .map(|rt| rt.size())
            .unwrap_or_else(window_size)
            .as_uvec2();

        let io_tex = self.textures.get_or_insert_mut(size, || {
            log::info!(
                "Creating PostProcess IOTextures with size: ({}, {})",
                size.x,
                size.y
            );
            InOutTextures::new(size).unwrap() // TODO maybe this is better to raise the error somehow?
        });

        // default sampler
        let sampler = if info.nearest_sampler {
            &self.nearest_sampler
        } else {
            &self.linear_sampler
        };

        if is_presenting_frame {
            let rt = self
                .frame_rt
                .as_ref()
                .ok_or_else(|| "PostFX Frame RenderTexture is not initiated.".to_string())?;

            let bg_key = BindGroupKey {
                tex: rt.id(),
                sampler: sampler.id(),
            };
            let bind_group = self.bind_groups.get_or_insert(bg_key, || {
                gfx::create_bind_group()
                    .with_label("PostProcess Frame BindGroup")
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
                .bindings(&[bind_group])
                .draw(0..6);

            // draw first pass
            gfx::render_to_texture(&io_tex.in_rt, &renderer)?;
        } else {
            // clear the input texture
            let mut renderer = Renderer::new();
            renderer.begin_pass().clear_color(Color::TRANSPARENT);
            gfx::render_to_texture(&io_tex.in_rt, &renderer)?;

            // draw first pass
            gfx::render_to_texture(&io_tex.in_rt, info.render)?;
        }

        // effect pass
        info.effects
            .iter()
            .filter(|f| f.is_enabled())
            .for_each(|effect| {
                let sampler = effect
                    .texture_filter()
                    .map(|tf| match tf {
                        TextureFilter::Linear => &self.linear_sampler,
                        TextureFilter::Nearest => &self.nearest_sampler,
                    })
                    .unwrap_or(sampler);

                // If necessary we need to insert on the cache the bind groups and then get them
                // to avoid borrow issues.
                insert_bg!(self, &io_tex.in_rt, sampler);
                insert_bg!(self, &io_tex.out_rt, sampler);
                insert_bg!(self, &io_tex.temp_rt, sampler);

                // get them
                let in_bg = get_bg!(self, &io_tex.in_rt, sampler);
                let out_bg = get_bg!(self, &io_tex.out_rt, sampler);
                let temp_bg = get_bg!(self, &io_tex.temp_rt, sampler);

                let data = IOPostFxData {
                    input: TextureBindGroup {
                        tex: &io_tex.in_rt,
                        bind_group: in_bg,
                    },
                    output: TextureBindGroup {
                        tex: &io_tex.out_rt,
                        bind_group: out_bg,
                    },
                    temp: TextureBindGroup {
                        tex: &io_tex.temp_rt,
                        bind_group: temp_bg,
                    },
                };

                match effect.apply(data) {
                    Ok(swap) => {
                        if swap {
                            io_tex.swap();
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Unable to apply post process effect '{}': {}",
                            effect.name(),
                            e
                        );
                    }
                }
            });

        // final pass
        let bg_key = BindGroupKey {
            tex: io_tex.in_rt.id(),
            sampler: sampler.id(),
        };
        let bind_group = self.bind_groups.get_or_insert(bg_key, || {
            gfx::create_bind_group()
                .with_label("PostProcess BindGroup")
                .with_layout(self.pip.bind_group_layout_ref(0).unwrap())
                .with_texture(0, io_tex.in_rt.texture())
                .with_sampler(1, sampler)
                .build()
                .unwrap() // TODO raise error...
        });

        let mut renderer = Renderer::new();
        renderer
            .begin_pass()
            .pipeline(&self.pip)
            .bindings(&[bind_group])
            .draw(0..6);

        match target {
            None => gfx::render_to_frame(&renderer),
            Some(t) => gfx::render_to_texture(t, &renderer),
        }
    }

    pub fn present_pfx_frame(
        &mut self,
        effects: &[&dyn PostFx],
        nearest: bool,
    ) -> Result<(), String> {
        debug_assert!(
            self.frame_rt.is_some(),
            "PostFX Frame RenderTexture is not initiated."
        );
        self.process(
            &PostProcess {
                effects,
                render: &Renderer::new(),
                nearest_sampler: nearest,
            },
            true,
            None,
        )
    }

    pub fn check_and_get_pfx_frame(&mut self) -> Result<&RenderTexture, String> {
        let size = window_size().as_uvec2();
        let rt = self
            .frame_rt
            .get_or_insert_with(|| create_frame_rt(size).unwrap());
        let tex_size = rt.size().as_uvec2();
        if tex_size != size {
            *rt = create_frame_rt(size)?;
        }

        Ok(rt)
    }
}

fn create_frame_rt(size: UVec2) -> Result<RenderTexture, String> {
    log::info!(
        "Created PostFX Frame RenderTexture size:{},{}",
        size.x,
        size.y
    );
    gfx::create_render_texture()
        .with_label("PostFX Frame RenderTexture")
        .with_size(size.x, size.y)
        .build()
}
