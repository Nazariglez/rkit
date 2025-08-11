use bytemuck::{Pod, Zeroable};
use corelib::{
    gfx::{
        self, BindGroup, BindGroupLayout, BindingType, BlendMode, Buffer, Color, RenderPipeline,
        Renderer, VertexFormat, VertexLayout, VertexStepMode,
    },
    math::{Mat3, Mat4, Vec2},
};
use encase::{ShaderType, UniformBuffer};

use super::utils::mat4_from_affine2d;

const SHADER: &str = include_str!("./circles.wgsl");

/// Uniform data used by the circle shader
#[derive(Debug, ShaderType)]
struct Locals {
    mvp: Mat4,
    fade_at: f32,
    antialias: f32,
    _pad: Vec2,
}

impl Locals {
    const fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Data for one circle instance on the GPU
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct GpuCircle {
    mode: u32,
    center: Vec2,
    out_radius: f32,
    in_color: Color,
    out_color: Color,
    in_radius: f32,
    start_angle: f32,
    end_angle: f32,
    progress: f32,
}

/// Different ways a circle can be drawn
#[derive(Debug, Clone, Copy)]
enum CircleMode {
    Fill {
        in_color: Color,
        out_color: Color,
    },
    Stroke {
        width: f32,
        color: Color,
    },
    LoadBar {
        start_angle: f32,
        end_angle: f32,
        progress: f32,
        width: f32,
        in_color: Color,
        out_color: Color,
    },
}

impl CircleMode {
    /// Converts the mode into GPU instance data
    fn to_gpu(self, center: Vec2, radius: f32) -> GpuCircle {
        match self {
            CircleMode::Fill {
                in_color,
                out_color,
            } => GpuCircle {
                mode: 0,
                center,
                out_radius: radius,
                in_color,
                out_color,
                in_radius: 0.0,
                start_angle: 0.0,
                end_angle: 0.0,
                progress: 0.0,
            },
            CircleMode::Stroke { width, color } => GpuCircle {
                mode: 1,
                center,
                out_radius: radius,
                in_color: Color::TRANSPARENT,
                out_color: color,
                in_radius: radius - width,
                start_angle: 0.0,
                end_angle: 0.0,
                progress: 0.0,
            },
            CircleMode::LoadBar {
                start_angle,
                end_angle,
                progress,
                width,
                in_color,
                out_color,
            } => GpuCircle {
                mode: 2,
                center,
                out_radius: radius,
                in_color,
                out_color,
                in_radius: radius - width,
                start_angle,
                end_angle,
                progress,
            },
        }
    }
}

/// Batches circles and draws them with instancing
pub struct CircleBatcher {
    pip: RenderPipeline,
    vbo: Buffer,
    ubo: Buffer,
    ubs: UniformBuffer<[u8; Locals::size()]>,
    bind_group: BindGroup,
    entities: Vec<GpuCircle>,
    locals: Locals,
    projection: Mat4,
    transform: Mat3,
    dirty_ubo: bool,
    dirty_vbo: bool,
}

impl CircleBatcher {
    pub fn new() -> Result<Self, String> {
        let shader = SHADER.replace(
            "{{SRGB_TO_LINEAR}}",
            include_str!("../../resources/to_linear.wgsl"),
        );

        let pip = gfx::create_render_pipeline(&shader)
            .with_vertex_layout(
                VertexLayout::new()
                    .with_step_mode(VertexStepMode::Instance)
                    .with_attr(0, VertexFormat::UInt32)
                    .with_attr(1, VertexFormat::Float32x2)
                    .with_attr(2, VertexFormat::Float32)
                    .with_attr(3, VertexFormat::Float32x4)
                    .with_attr(4, VertexFormat::Float32x4)
                    .with_attr(5, VertexFormat::Float32)
                    .with_attr(6, VertexFormat::Float32)
                    .with_attr(7, VertexFormat::Float32)
                    .with_attr(8, VertexFormat::Float32),
            )
            .with_bind_group_layout(
                BindGroupLayout::new().with_entry(
                    BindingType::uniform(0)
                        .with_vertex_visibility(true)
                        .with_fragment_visibility(true),
                ),
            )
            .with_blend_mode(BlendMode::NORMAL)
            .build()?;

        let vbo = gfx::create_vertex_buffer(&[] as &[f32])
            .with_write_flag(true)
            .build()?;

        let projection = Mat4::orthographic_rh(0.0, 800.0, 600.0, 0.0, 0.0, 1.0);
        let locals = Locals {
            mvp: projection,
            fade_at: 0.9,
            antialias: 0.0,
            _pad: Vec2::ZERO,
        };

        let mut ubs = UniformBuffer::new([0; Locals::size()]);
        ubs.write(&locals).map_err(|e| e.to_string()).unwrap();

        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_layout(pip.bind_group_layout_ref(0)?)
            .with_uniform(0, &ubo)
            .build()?;

        Ok(CircleBatcher {
            pip,
            vbo,
            ubo,
            ubs,
            bind_group,
            locals,
            entities: vec![],
            projection,
            transform: Mat3::IDENTITY,
            dirty_ubo: true,
            dirty_vbo: true,
        })
    }

    /// Sets the projection matrix
    pub fn set_projection(&mut self, projection: Mat4) {
        self.projection = projection;
        self.dirty_ubo = true;
    }

    /// Sets the transform matrix
    pub fn set_transform(&mut self, transform: Mat3) {
        self.transform = transform;
        self.dirty_ubo = true;
    }

    /// Enables or disables antialiasing
    pub fn set_antialias(&mut self, aa: bool) {
        self.locals.antialias = if aa { 1.0 } else { 0.0 };
        self.dirty_ubo = true;
    }

    /// Removes all circles from the batch
    pub fn clear(&mut self) {
        self.entities.clear();
        self.dirty_vbo = true;
    }

    /// Uploads changes to GPU buffers
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
                .with_data(&self.entities)
                .build()?;
        }

        Ok(())
    }

    /// Adds a filled circle
    #[inline]
    pub fn fill<'a>(&'a mut self, center: Vec2, radius: f32) -> CircleFillBuilder<'a> {
        CircleFillBuilder {
            batcher: self,
            center,
            radius,
            mode: CircleMode::Fill {
                in_color: Color::WHITE,
                out_color: Color::WHITE,
            },
        }
    }

    /// Adds a stroked circle
    #[inline]
    pub fn stroke<'a>(&'a mut self, center: Vec2, radius: f32) -> CircleStrokeBuilder<'a> {
        CircleStrokeBuilder {
            batcher: self,
            center,
            radius,
            mode: CircleMode::Stroke {
                width: 1.0,
                color: Color::WHITE,
            },
        }
    }

    /// Adds an arc circle
    #[inline]
    pub fn arc<'a>(&'a mut self, center: Vec2, radius: f32) -> CircleArcBuilder<'a> {
        CircleArcBuilder {
            batcher: self,
            center,
            radius,
            mode: CircleMode::LoadBar {
                start_angle: 0.0,
                end_angle: std::f32::consts::TAU,
                progress: 1.0,
                width: 1.0,
                in_color: Color::WHITE,
                out_color: Color::WHITE,
            },
        }
    }

    /// Adds a load bar circle
    #[inline]
    pub fn load_bar<'a>(&'a mut self, center: Vec2, radius: f32) -> CircleLoaderBuilder<'a> {
        CircleLoaderBuilder {
            batcher: self,
            center,
            radius,
            mode: CircleMode::LoadBar {
                start_angle: 0.0,
                end_angle: std::f32::consts::TAU,
                progress: 1.0,
                width: 1.0,
                in_color: Color::WHITE,
                out_color: Color::WHITE,
            },
        }
    }

    /// Draws all circles into a renderer pass
    #[inline]
    pub fn apply_pass_to<'a>(&'a self, renderer: &mut Renderer<'a>) {
        debug_assert!(
            !(self.dirty_vbo && self.dirty_ubo),
            "CircleBatcher is dirty, call 'upload' before apply pass to a renderer."
        );
        renderer
            .begin_pass()
            .pipeline(&self.pip)
            .buffers(&[&self.vbo])
            .bindings(&[&self.bind_group])
            .draw_instanced(0..6, self.entities.len() as _);
    }
}

pub struct CircleFillBuilder<'a> {
    batcher: &'a mut CircleBatcher,
    center: Vec2,
    radius: f32,
    mode: CircleMode,
}

impl CircleFillBuilder<'_> {
    #[inline]
    pub fn color(mut self, color: Color) -> Self {
        match &mut self.mode {
            CircleMode::Fill {
                in_color,
                out_color,
            } => {
                *in_color = color;
                *out_color = color;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn inner_color(mut self, color: Color) -> Self {
        match &mut self.mode {
            CircleMode::Fill { in_color, .. } => {
                *in_color = color;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn outer_color(mut self, color: Color) -> Self {
        match &mut self.mode {
            CircleMode::Fill { out_color, .. } => {
                *out_color = color;
            }
            _ => unreachable!(),
        }
        self
    }
}

impl<'a> Drop for CircleFillBuilder<'a> {
    fn drop(&mut self) {
        self.batcher
            .entities
            .push(self.mode.to_gpu(self.center, self.radius));
    }
}

pub struct CircleStrokeBuilder<'a> {
    batcher: &'a mut CircleBatcher,
    center: Vec2,
    radius: f32,
    mode: CircleMode,
}

impl CircleStrokeBuilder<'_> {
    #[inline]
    pub fn color(mut self, c: Color) -> Self {
        match &mut self.mode {
            CircleMode::Stroke { color, .. } => {
                *color = c;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn width(mut self, w: f32) -> Self {
        match &mut self.mode {
            CircleMode::Stroke { width, .. } => {
                *width = w;
            }
            _ => unreachable!(),
        }
        self
    }
}

impl<'a> Drop for CircleStrokeBuilder<'a> {
    fn drop(&mut self) {
        self.batcher
            .entities
            .push(self.mode.to_gpu(self.center, self.radius));
    }
}

pub struct CircleArcBuilder<'a> {
    batcher: &'a mut CircleBatcher,
    center: Vec2,
    radius: f32,
    mode: CircleMode,
}

impl CircleArcBuilder<'_> {
    #[inline]
    pub fn color(mut self, c: Color) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar {
                in_color,
                out_color,
                ..
            } => {
                *in_color = c;
                *out_color = c;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn width(mut self, w: f32) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar { width, .. } => {
                *width = w;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn start_angle(mut self, angle: f32) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar { start_angle, .. } => {
                *start_angle = angle;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn end_angle(mut self, angle: f32) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar { end_angle, .. } => {
                *end_angle = angle;
            }
            _ => unreachable!(),
        }
        self
    }
}

impl<'a> Drop for CircleArcBuilder<'a> {
    fn drop(&mut self) {
        self.batcher
            .entities
            .push(self.mode.to_gpu(self.center, self.radius));
    }
}

pub struct CircleLoaderBuilder<'a> {
    batcher: &'a mut CircleBatcher,
    center: Vec2,
    radius: f32,
    mode: CircleMode,
}

impl CircleLoaderBuilder<'_> {
    #[inline]
    pub fn color(mut self, c: Color) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar {
                in_color,
                out_color,
                ..
            } => {
                *in_color = c;
                *out_color = c;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn inner_color(mut self, c: Color) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar { in_color, .. } => {
                *in_color = c;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn outer_color(mut self, c: Color) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar { out_color, .. } => {
                *out_color = c;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn progress(mut self, value: f32) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar { progress, .. } => {
                *progress = value.clamp(0.0, 1.0);
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn width(mut self, w: f32) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar { width, .. } => {
                *width = w;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn start_angle(mut self, angle: f32) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar { start_angle, .. } => {
                *start_angle = angle;
            }
            _ => unreachable!(),
        }
        self
    }

    #[inline]
    pub fn end_angle(mut self, angle: f32) -> Self {
        match &mut self.mode {
            CircleMode::LoadBar { end_angle, .. } => {
                *end_angle = angle;
            }
            _ => unreachable!(),
        }
        self
    }
}

impl<'a> Drop for CircleLoaderBuilder<'a> {
    fn drop(&mut self) {
        self.batcher
            .entities
            .push(self.mode.to_gpu(self.center, self.radius));
    }
}
