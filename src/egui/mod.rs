use crate::ecs::prelude::*;
use crate::math::Vec2;
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use corelib::{
    app::{window_dpi_scale, window_size},
    gfx::{
        self, BindGroupLayout, BindingType, BlendMode, Buffer, Color, IndexFormat, RenderPipeline,
        Renderer, Texture, TextureFormat, VertexFormat,
    },
};
use egui::epaint::ClippedShape;
pub use egui::*;
use encase::{ShaderType, UniformBuffer};
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use std::cell::RefCell;

pub(crate) static EGUI_PAINTER: Lazy<AtomicRefCell<EguiPainter>> = Lazy::new(|| {
    // corelib::app::on_sys_post_update(clean_2d);

    AtomicRefCell::new(EguiPainter::default())
});

fn get_egui_painter() -> AtomicRef<'static, EguiPainter> {
    EGUI_PAINTER.borrow()
}

fn get_mut_egui_painter() -> AtomicRefMut<'static, EguiPainter> {
    EGUI_PAINTER.borrow_mut()
}

#[derive(Default, Copy, Clone, Debug, PartialEq, ShaderType)]
struct EguiLocals {
    screen_size_in_points: Vec2,
    dithering: u32,
    _pading: u32,
}

struct EguiPainter {
    pipeline: RenderPipeline,
    vbo: Buffer,
    ubo: Buffer,
    ubs: UniformBuffer<[u8; 16]>,
    ebo: Buffer,

    textures: FxHashMap<TextureId, Texture>,
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
            .find(|t| matches!(t, TextureFormat::Rgba8UNorm | TextureFormat::Bgra8UNorm))
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

        let pipeline = gfx::create_render_pipeline(include_str!("./egui.wgsl"))
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
            .with_blend_mode(BlendMode::NORMAL)
            .with_compatible_texture(target_format)
            .with_fragment_entry(fs_entry)
            .with_primitive(gfx::Primitive::TriangleStrip)
            .build()
            .unwrap();

        Self {
            pipeline,
            vbo,
            ubo,
            ubs,
            ebo,
            textures: FxHashMap::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct EguiPlugin {}

impl Plugin for EguiPlugin {
    fn apply(&self, app: &mut App) {
        let ctx = EguiContext {
            ctx: Context::default(),
            raw_input: RawInput::default(),
            clear_color: None,
        };

        app.add_resource(ctx);
    }
}

#[derive(Resource)]
pub struct EguiContext {
    ctx: Context,
    raw_input: RawInput,
    clear_color: Option<Color>,
}

impl EguiContext {
    pub fn clear(&mut self, color: Color) -> &mut Self {
        self.clear_color = Some(color);
        self
    }

    pub fn run<F>(&mut self, cb: F) -> EguiDraw
    where
        F: FnMut(&Context),
    {
        let FullOutput {
            platform_output: _,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output,
        } = self.ctx.run(self.raw_input.take(), cb);
        let needs_update_textures = !textures_delta.is_empty();
        let needs_repaint = needs_update_textures
            || viewport_output
                .values()
                .any(|out| out.repaint_delay.is_zero());

        let primitives = self.ctx.tessellate(shapes, pixels_per_point);

        // TODO: process platform_output

        EguiDraw {
            clear: self.clear_color,
            ctx: self.ctx.clone(),
            textures_delta,
            primitives,
            needs_repaint,
            pixels_per_point,
        }
    }
}

pub struct EguiDraw {
    clear: Option<Color>,
    ctx: Context,
    textures_delta: TexturesDelta,
    primitives: Vec<ClippedPrimitive>,
    needs_repaint: bool,
    pixels_per_point: f32,
}

impl gfx::AsRenderer for EguiDraw {
    fn render(&self, target: Option<&gfx::RenderTexture>) -> Result<(), String> {
        let mut painter = get_mut_egui_painter();

        let screen_size = target.map(|t| t.size()).unwrap_or_else(window_size);

        // //
        // let screen_size_in_points = screen_size / window_dpi_scale();

        painter
            .ubs
            .write(&EguiLocals {
                screen_size_in_points: screen_size,
                dithering: u32::from(false),
                _pading: 0,
            })
            .map_err(|e| e.to_string())?;

        gfx::write_buffer(&painter.ubo)
            .with_data(painter.ubs.as_ref())
            .build()?;

        // TODO: update textures delta here

        let mut renderer = Renderer::new();
        let pass = renderer.begin_pass();
        if let Some(color) = self.clear {
            pass.clear_color(color.as_linear());
        }

        self.flush(&renderer, target)
    }
}
