use crate::{
    gfx::{
        self, BindGroup, BindGroupLayout, BindGroupLayoutRef, BindingType, BlendComponent,
        BlendFactor, BlendMode, BlendOperation, Buffer, Color, IndexFormat, RenderPipeline,
        RenderTexture, Renderer, Sampler, Texture, TextureFilter, TextureWrap, VertexFormat,
    },
    math::{UVec2, Vec2},
};
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use egui::{
    ClippedPrimitive, ImageData, Mesh, Rect, TextureId, TextureOptions,
    epaint::{self, ImageDelta},
};
use encase::{ShaderType, UniformBuffer};
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;

pub(super) static EGUI_PAINTER: Lazy<AtomicRefCell<EguiPainter>> =
    Lazy::new(|| AtomicRefCell::new(EguiPainter::default()));

pub(super) fn get_egui_painter() -> AtomicRef<'static, EguiPainter> {
    EGUI_PAINTER.borrow()
}

pub(super) fn get_mut_egui_painter() -> AtomicRefMut<'static, EguiPainter> {
    EGUI_PAINTER.borrow_mut()
}

struct CachedTexture {
    tex: Texture,
    sampler: Sampler,
    bind: BindGroup,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, ShaderType)]
pub(super) struct EguiLocals {
    screen_size_in_points: Vec2,
    dithering: u32,
    _pading: u32,
}

pub(super) struct EguiPainter {
    pipeline: RenderPipeline,
    vbo: Buffer,
    ebo: Buffer,
    ubo: Buffer,
    ubs: UniformBuffer<[u8; 16]>,
    ubo_bind: BindGroup,
    textures: FxHashMap<TextureId, CachedTexture>,
}

impl Default for EguiPainter {
    fn default() -> Self {
        let mut ubs = UniformBuffer::new([0; 16]);
        ubs.write(&EguiLocals::default()).unwrap();

        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
            .with_label("EguiPainter UBO Transform")
            .with_write_flag(true)
            .build()
            .unwrap();

        let vbo = gfx::create_vertex_buffer(&[] as &[f32])
            .with_label("EguiPainter VBO")
            .with_write_flag(true)
            .build()
            .unwrap();

        let ebo = gfx::create_index_buffer(&[] as &[u32])
            .with_label("EguiPainter EBO")
            .with_write_flag(true)
            .build()
            .unwrap();

        let surface_formats = gfx::limits().surface_formats;
        let target_format = surface_formats
            .iter()
            // .find(|t| matches!(t, TextureFormat::Rgba8UNorm | TextureFormat::Bgra8UNorm))
            .find(|t| t.is_srgb())
            .or_else(|| surface_formats.first())
            .cloned()
            .unwrap();

        let fs_entry = if target_format.is_srgb() {
            log::warn!(
                "Detected a linear (sRGBA aware) framebuffer {target_format:?}. egui prefers Rgba8Unorm or Bgra8Unorm"
            );
            "fs_main_linear_framebuffer"
        } else {
            "fs_main_gamma_framebuffer"
        };

        log::debug!("TODO: remove me -> selected format {target_format:?} {fs_entry:?}");

        let pipeline = gfx::create_render_pipeline(include_str!("./egui.wgsl"))
            .with_label("Egui RenderPipeline")
            .with_vertex_layout(
                gfx::VertexLayout::new()
                    .with_attr(0, VertexFormat::Float32x2)
                    .with_attr(1, VertexFormat::Float32x2)
                    .with_attr(2, VertexFormat::UInt32),
            )
            .with_bind_group_layout(
                BindGroupLayout::new().with_entry(
                    BindingType::uniform(0)
                        .with_vertex_visibility(true)
                        .with_fragment_visibility(true),
                ),
            )
            .with_bind_group_layout(
                BindGroupLayout::new()
                    .with_entry(BindingType::texture(0).with_fragment_visibility(true))
                    .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
            )
            .with_index_format(IndexFormat::UInt32)
            .with_blend_mode(BlendMode {
                color: BlendComponent {
                    src: BlendFactor::One,
                    dst: BlendFactor::InverseSourceAlpha,
                    op: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src: BlendFactor::InverseDestinationAlpha,
                    dst: BlendFactor::One,
                    op: BlendOperation::Add,
                },
            })
            // .with_compatible_texture(target_format)
            .with_fragment_entry(fs_entry)
            .with_primitive(gfx::Primitive::Triangles)
            .build()
            .unwrap();

        let ubo_bind = gfx::create_bind_group()
            .with_label("EguiPainter UBO BindGroup")
            .with_layout(pipeline.bind_group_layout_ref(0).unwrap())
            .with_uniform(0, &ubo)
            .build()
            .unwrap();

        Self {
            pipeline,
            vbo,
            ebo,
            ubo,
            ubs,
            ubo_bind,
            textures: FxHashMap::default(),
        }
    }
}

impl EguiPainter {
    pub fn paint(
        &mut self,
        clear_color: Option<Color>,
        primitives: &[ClippedPrimitive],
        target: Option<&gfx::RenderTexture>,
        win_size: Vec2,
        win_dpi: f32,
    ) -> Result<(), String> {
        let screen_size = target.map(|t| t.size()).unwrap_or(win_size);

        // FIXME: everthing should be done in one renderer with multuple passes
        self.ubs
            .write(&EguiLocals {
                screen_size_in_points: screen_size,
                dithering: u32::from(false),
                _pading: 0,
            })
            .map_err(|e| e.to_string())?;

        gfx::write_buffer(&self.ubo)
            .with_data(self.ubs.as_ref())
            .build()?;

        if let Some(color) = clear_color {
            let mut renderer = Renderer::new();
            renderer.begin_pass().clear_color(color);

            gfx::render_to_frame(&renderer)?;
        }

        for ClippedPrimitive {
            clip_rect,
            primitive,
        } in primitives.iter()
        {
            //
            match primitive {
                epaint::Primitive::Mesh(mesh) => {
                    self.paint_mesh(mesh, clip_rect, target, win_size, win_dpi)?
                }
                epaint::Primitive::Callback(paint_callback) => {
                    log::warn!("Egui CALLBACK unimplemented.");
                }
            }
        }

        Ok(())
    }

    fn paint_mesh(
        &mut self,
        mesh: &Mesh,
        clip_rect: &Rect,
        target: Option<&RenderTexture>,
        win_size: Vec2,
        win_dpi: f32,
    ) -> Result<(), String> {
        //
        let tex_bind = self
            .textures
            .get(&mesh.texture_id)
            .ok_or_else(|| format!("Invalid Egui texture id '{:?}'", mesh.texture_id))?;

        gfx::write_buffer(&self.vbo)
            .with_data(&mesh.vertices)
            .build()?;

        gfx::write_buffer(&self.ebo)
            .with_data(&mesh.indices)
            .build()?;

        let target_size = target.map_or(win_size, |t| t.size());
        let [sx, sy, sw, sh] = scissor_rect(clip_rect, win_dpi, target_size);
        if sw == 0 || sh == 0 {
            return Ok(());
        }

        let mut renderer = Renderer::new();
        renderer
            .begin_pass()
            .scissors(sx, sy, sw, sh)
            .pipeline(&self.pipeline)
            .bindings(&[&self.ubo_bind, &tex_bind.bind])
            .buffers(&[&self.vbo, &self.ebo])
            .draw(0..mesh.indices.len() as u32);

        match target {
            Some(rt) => gfx::render_to_texture(rt, &renderer),
            None => gfx::render_to_frame(&renderer),
        }
    }

    // pub fn add_texture(&mut self, tex: &Texture) -> SizedTexture {
    // // TODO: instead of texture we may need a sprite to know which sampler to use?
    // // similar to draw2d?
    //
    //     let id = TextureId::User(tex.id().into());
    //     let size = tex.size();
    //     self.textures.insert(id, tex.clone());
    //     SizedTexture {
    //         id,
    //         size: egui::Vec2::new(size.x, size.y),
    //     }
    // }

    pub fn set_texture(&mut self, id: TextureId, delta: &ImageDelta) {
        let [width, height] = delta.image.size();

        // update texture
        if let Some([x, y]) = delta.pos {
            let cached = self.textures.entry(id).or_insert_with(|| {
                let sampler = create_sampler_from(&delta.options);
                let tex = empty_texture(width as _, height as _);
                let bind = bind_group_from(
                    &tex,
                    &sampler,
                    self.pipeline.bind_group_layout_ref(1).unwrap(),
                );
                CachedTexture { tex, sampler, bind }
            });

            match &delta.image {
                ImageData::Color(image) => {
                    debug_assert_eq!(
                        image.width() * image.height(),
                        image.pixels.len(),
                        "Mismatch between texture size and texel count"
                    );

                    let data = bytemuck::cast_slice(&image.pixels);
                    update_texture(
                        &mut cached.tex,
                        x as _,
                        y as _,
                        width as _,
                        height as _,
                        data,
                    );
                }
            }

            return;
        }

        // create a new texture
        let tex = match &delta.image {
            egui::ImageData::Color(image) => {
                debug_assert_eq!(
                    image.width() * image.height(),
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );

                let data = bytemuck::cast_slice(&image.pixels);
                create_texture(data, width as _, height as _)
            }
        };

        let sampler = create_sampler_from(&delta.options);
        let bind = bind_group_from(
            &tex,
            &sampler,
            self.pipeline.bind_group_layout_ref(1).unwrap(),
        );
        self.textures
            .insert(id, CachedTexture { tex, sampler, bind });
    }

    pub fn remove_texture(&mut self, id: impl Into<TextureId>) {
        self.textures.remove(&id.into());
    }
}

fn bind_group_from(tex: &Texture, sampler: &Sampler, layout: &BindGroupLayoutRef) -> BindGroup {
    gfx::create_bind_group()
        .with_label("EguiPainter Texture BindGroup")
        .with_layout(layout)
        .with_texture(0, tex)
        .with_sampler(1, sampler)
        .build()
        .unwrap()
}

fn create_texture(data: &[u8], width: u32, height: u32) -> Texture {
    gfx::create_texture()
        .with_label("EguiPainter Texture")
        .from_bytes(data, width, height)
        // .with_format(TextureFormat::Rgba8UNorm)
        .with_write_flag(true)
        .build()
        .unwrap()
}

fn create_sampler_from(opts: &TextureOptions) -> Sampler {
    let filter = |tf| match tf {
        egui::TextureFilter::Nearest => TextureFilter::Nearest,
        egui::TextureFilter::Linear => TextureFilter::Linear,
    };
    let wrap = match opts.wrap_mode {
        egui::TextureWrapMode::ClampToEdge => TextureWrap::Clamp,
        egui::TextureWrapMode::Repeat => TextureWrap::Repeat,
        egui::TextureWrapMode::MirroredRepeat => TextureWrap::MirrorRepeat,
    };

    gfx::create_sampler()
        .with_mag_filter(filter(opts.magnification))
        .with_min_filter(filter(opts.minification))
        .with_wrap_x(wrap)
        .with_wrap_y(wrap)
        .build()
        .unwrap()
}

fn empty_texture(width: u32, height: u32) -> Texture {
    gfx::create_texture()
        .with_label("EguiPainter Texture")
        .with_empty_size(width, height)
        .with_write_flag(true)
        // .with_format(TextureFormat::Rgba8UNorm)
        .build()
        .unwrap()
}

fn update_texture(tex: &mut Texture, x: u32, y: u32, width: u32, height: u32, data: &[u8]) {
    gfx::write_texture(tex)
        .with_offset(UVec2::new(x, y))
        .with_size(UVec2::new(width, height))
        .from_data(data)
        .build()
        .unwrap();
}

fn scissor_rect(clip_rect: &egui::Rect, pixels_per_point: f32, target_size: Vec2) -> [u32; 4] {
    let t_size = (target_size * pixels_per_point).as_uvec2();

    let clip_min_x = pixels_per_point * clip_rect.min.x;
    let clip_min_y = pixels_per_point * clip_rect.min.y;
    let clip_max_x = pixels_per_point * clip_rect.max.x;
    let clip_max_y = pixels_per_point * clip_rect.max.y;

    let clip_min_x = clip_min_x.round() as u32;
    let clip_min_y = clip_min_y.round() as u32;
    let clip_max_x = clip_max_x.round() as u32;
    let clip_max_y = clip_max_y.round() as u32;

    let clip_min_x = clip_min_x.clamp(0, t_size.x);
    let clip_min_y = clip_min_y.clamp(0, t_size.y);
    let clip_max_x = clip_max_x.clamp(clip_min_x, t_size.x);
    let clip_max_y = clip_max_y.clamp(clip_min_y, t_size.y);

    let x = clip_min_x;
    let y = clip_min_y;
    let width = clip_max_x - clip_min_x;
    let height = clip_max_y - clip_min_y;

    [x, y, width, height]
}
