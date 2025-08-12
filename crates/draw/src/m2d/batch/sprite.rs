use bytemuck::{Pod, Zeroable};
use corelib::{
    gfx::{
        self, BindGroup, BindGroupLayout, BindingType, BlendMode, Buffer, Color, RenderPipeline,
        Renderer, VertexFormat, VertexLayout, VertexStepMode,
    },
    math::{Mat3, Mat4, Vec2, Vec3},
};
use encase::{ShaderType, UniformBuffer};
use rustc_hash::FxHashMap;
use std::ops::Range;

use crate::{CachedBindGroup, Sprite, SpriteId, batch::utils::mat4_from_affine2d};

const SHADER: &str = include_str!("./sprites.wgsl");

#[derive(Debug, ShaderType)]
struct Locals {
    mvp: Mat4,
}

impl Locals {
    const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
struct GpuSprite {
    center: Vec2,
    half_size: Vec2,
    color: Color,
    uv_pos: Vec2,
    uv_size: Vec2,
    rotation: f32,
    _pad: Vec3,
}

pub struct SpriteBatcher {
    pip: RenderPipeline,

    ubo: Buffer,
    ubs: UniformBuffer<[u8; Locals::size()]>,
    bind_locals: BindGroup,
    locals: Locals,
    projection: Mat4,
    transform: Mat3,
    dirty_ubo: bool,

    vbo: Buffer,
    dirty_vbo: bool,

    sprites_cache: FxHashMap<SpriteId, CachedBindGroup>,

    last_sprite: Option<SpriteId>,
    sprites: Vec<GpuSprite>,
    batches: Vec<(SpriteId, Range<u64>)>,
}

impl SpriteBatcher {
    pub fn new() -> Result<Self, String> {
        let shader = SHADER.replace(
            "{{SRGB_TO_LINEAR}}",
            include_str!("../../resources/to_linear.wgsl"),
        );

        let pip = gfx::create_render_pipeline(&shader)
            .with_label("SpriteBatcher RenderPipeline")
            .with_vertex_layout(
                VertexLayout::new()
                    .with_step_mode(VertexStepMode::Instance)
                    .with_attr(0, VertexFormat::Float32x2)
                    .with_attr(1, VertexFormat::Float32x2)
                    .with_attr(2, VertexFormat::Float32x4)
                    .with_attr(3, VertexFormat::Float32x2)
                    .with_attr(4, VertexFormat::Float32x2)
                    .with_attr(5, VertexFormat::Float32)
                    .with_attr(6, VertexFormat::Float32x3),
            )
            .with_bind_group_layout(
                BindGroupLayout::new()
                    .with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
            )
            .with_bind_group_layout(
                BindGroupLayout::new()
                    .with_entry(BindingType::texture(0).with_fragment_visibility(true))
                    .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
            )
            .with_blend_mode(BlendMode::NORMAL)
            .build()?;

        let vbo = gfx::create_vertex_buffer(&[] as &[f32])
            .with_label("SpriteBatcher VBO")
            .with_write_flag(true)
            .build()?;

        let projection = Mat4::orthographic_rh(0.0, 800.0, 600.0, 0.0, 0.0, 1.0);
        let locals = Locals { mvp: projection };
        let mut ubs = UniformBuffer::new([0; Locals::size()]);
        ubs.write(&locals).map_err(|e| e.to_string())?;
        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
            .with_label("SpriteBatcher UBO")
            .with_write_flag(true)
            .build()?;

        let bind_locals = gfx::create_bind_group()
            .with_label("SpriteBatcher BindGroup")
            .with_layout(pip.bind_group_layout_ref(0)?)
            .with_uniform(0, &ubo)
            .build()?;

        Ok(Self {
            pip,
            ubo,
            ubs,
            bind_locals,
            locals,
            projection,
            transform: Mat3::IDENTITY,
            dirty_ubo: true,
            vbo,
            dirty_vbo: true,
            sprites_cache: FxHashMap::default(),
            last_sprite: None,
            sprites: vec![],
            batches: vec![],
        })
    }

    #[inline]
    pub fn set_projection(&mut self, mvp: Mat4) {
        self.projection = mvp;
        self.dirty_ubo = true;
    }

    #[inline]
    pub fn set_transform(&mut self, matrix: Mat3) {
        self.transform = matrix;
        self.dirty_ubo = true;
    }

    #[inline]
    pub fn clear(&mut self) {
        self.sprites_cache.retain(|_k, v| !v.expired());
        self.batches.clear();
        self.sprites.clear();
        self.last_sprite = None;
        self.dirty_vbo = true;
    }

    #[inline]
    pub fn sprite(&mut self, sprite: &Sprite, center: Vec2) -> GpuSpriteBuilder<'_> {
        let is_active_batch = self.last_sprite.is_some_and(|id| sprite.id() == id);
        if !is_active_batch {
            // generate bind group for it
            let _ = self.cached_bind_group_for(sprite);
            self.last_sprite = Some(sprite.id());
            let slen = self.sprites.len() as u64;
            self.batches.push((sprite.id(), slen..slen + 1));
        } else {
            // increase range if we're in the same batch
            match self.batches.last_mut() {
                Some((_, range)) => {
                    range.end += 1;
                }
                None => unreachable!(),
            }
        }

        let frame = sprite.frame();
        let tex_size = sprite.texture().size();
        let uv_pos = frame.min() / tex_size;
        let uv_size = frame.size / tex_size;

        GpuSpriteBuilder {
            batcher: self,
            size: sprite.size(),
            sprite: GpuSprite {
                center,
                half_size: sprite.size() * 0.5,
                rotation: 0.0,
                color: Color::WHITE,
                uv_pos,
                uv_size,
                _pad: Vec3::ZERO,
            },
        }
    }

    pub fn upload(&mut self) -> Result<(), String> {
        if self.dirty_ubo {
            self.dirty_ubo = false;
            self.locals.mvp = self.projection * mat4_from_affine2d(self.transform);
            self.ubs.write(&self.locals).map_err(|e| e.to_string())?;
            gfx::write_buffer(&self.ubo)
                .with_data(self.ubs.as_ref())
                .build()?;
        }

        if self.dirty_vbo {
            self.dirty_vbo = false;
            gfx::write_buffer(&self.vbo)
                .with_data(&self.sprites)
                .build()?;
        }

        Ok(())
    }

    pub fn apply_pass_to<'a>(&'a self, renderer: &mut Renderer<'a>) {
        self.batches
            .iter()
            .for_each(|(id, range)| match self.sprites_cache.get(id) {
                Some(bind_group) => {
                    let count = range.end - range.start;
                    let stride = std::mem::size_of::<GpuSprite>() as u64;
                    let start = range.start * stride;
                    let end = range.end * stride;

                    renderer
                        .begin_pass()
                        .pipeline(&self.pip)
                        .buffers_with_offset(&[(&self.vbo, start..end)])
                        .bindings(&[&self.bind_locals, &bind_group.bind])
                        .draw_instanced(0..6, count as u32);
                }
                None => unreachable!(),
            });
    }

    fn cached_bind_group_for(&mut self, sprite: &Sprite) -> BindGroup {
        self.sprites_cache
            .entry(sprite.id())
            .or_insert_with(|| {
                let bind = gfx::create_bind_group()
                    .with_layout(self.pip.bind_group_layout_ref(1).unwrap())
                    .with_texture(0, sprite.texture())
                    .with_sampler(1, sprite.sampler())
                    .build()
                    .unwrap();

                let signal = sprite.drop_observer.signal();
                CachedBindGroup { signal, bind }
            })
            .bind
            .clone()
    }
}

pub struct GpuSpriteBuilder<'a> {
    batcher: &'a mut SpriteBatcher,
    size: Vec2,
    sprite: GpuSprite,
}

impl<'a> GpuSpriteBuilder<'a> {
    #[inline]
    pub fn color(mut self, c: Color) -> Self {
        self.sprite.color = c;
        self
    }

    #[inline]
    pub fn rotation(mut self, angle: f32) -> Self {
        self.sprite.rotation = angle;
        self
    }

    #[inline]
    pub fn scale(mut self, scale: Vec2) -> Self {
        self.sprite.half_size = self.size * scale * 0.5;
        self
    }
}

impl<'a> Drop for GpuSpriteBuilder<'a> {
    fn drop(&mut self) {
        self.batcher.sprites.push(self.sprite);
        self.batcher.dirty_vbo = true;
    }
}
