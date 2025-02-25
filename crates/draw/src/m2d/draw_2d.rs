use super::{get_2d_painter, get_mut_2d_painter};
use crate::m2d::images::Image2D;
use crate::m2d::mat3_stack::Mat3Stack;
use crate::m2d::painter::DrawPipelineId;
use crate::m2d::shapes::{Line2D, Path2D, Rectangle2D, Triangle2D};
use crate::m2d::text::Text2D;
use crate::sprite::Sprite;
use crate::text::get_mut_text_system;
use crate::{BaseCam2D, Circle2D, Ellipse2D, Pattern2D, Polygon2D, Star2D};
use arrayvec::ArrayVec;
use corelib::gfx::consts::MAX_BIND_GROUPS_PER_PIPELINE;
use corelib::gfx::{self, AsRenderer, BindGroup, Color, RenderPipeline, RenderTexture, Renderer};
use corelib::math::{Mat3, Mat4, Rect, Vec2, vec2, vec3, vec4};
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut, Range};

// TODO Cached elements is a must

// This is used to avoid heap allocations when doing small number of drawcalls
const STACK_ALLOCATED_QUADS: usize = 200;

#[derive(Clone)]
pub struct PipelineContext {
    pub pipeline: RenderPipeline,
    pub groups: ArrayVec<BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>,
    pub vertex_offset: usize,
    pub x_pos: usize,
    pub y_pos: usize,
    pub alpha_pos: Option<usize>,
}

pub trait AsBindGroups {
    fn to_bind_groups(self) -> ArrayVec<BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>;
}

impl AsBindGroups for &[BindGroup] {
    fn to_bind_groups(self) -> ArrayVec<BindGroup, MAX_BIND_GROUPS_PER_PIPELINE> {
        debug_assert!(
            self.len() <= MAX_BIND_GROUPS_PER_PIPELINE,
            "Bind Groups must be less than {}",
            MAX_BIND_GROUPS_PER_PIPELINE
        );
        (self as &[_]).try_into().unwrap()
    }
}

struct BatchInfo {
    vbo_range: Range<u64>,
    ebo_range: Range<u64>,
    start_idx: usize,
    end_idx: usize,
    pipeline: RenderPipeline,
    bind_groups: ArrayVec<BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>,
}

impl Clone for BatchInfo {
    fn clone(&self) -> Self {
        Self {
            vbo_range: self.vbo_range.clone(),
            ebo_range: self.ebo_range.clone(),
            start_idx: self.start_idx,
            end_idx: self.end_idx,
            pipeline: self.pipeline.clone(),
            bind_groups: self.bind_groups.clone(),
        }
    }
}

impl BatchInfo {
    fn is_compatible(&self, other: &Self) -> bool {
        if self.pipeline != other.pipeline {
            return false;
        }

        if self.bind_groups != other.bind_groups {
            return false;
        }

        true
    }

    fn count(&self) -> usize {
        self.end_idx - self.start_idx
    }
}

pub struct Drawing<'a, T>
where
    T: Element2D,
{
    inner: Option<T>,
    draw: &'a mut Draw2D,
}

impl<'a, T> Drawing<'a, T>
where
    T: Element2D,
{
    pub fn new(draw: &'a mut Draw2D, inner: T) -> Self {
        Self {
            inner: Some(inner),
            draw,
        }
    }

    pub fn into_inner(mut self) -> T {
        self.inner.take().unwrap()
    }
}

impl<T> Drop for Drawing<'_, T>
where
    T: Element2D,
{
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            self.draw.add_element(&inner);
        }
    }
}

impl<T> Deref for Drawing<'_, T>
where
    T: Element2D,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<T> DerefMut for Drawing<'_, T>
where
    T: Element2D,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct DrawStats {
    pub elements: usize,
    pub batches: usize,
}

#[derive(Default, Clone)]
pub struct Draw2D {
    round_pixels: bool,

    size: Vec2,

    projection: Mat4,
    inverse_projection: Mat4,
    inverse_transform: Option<Mat3>,

    clear_color: Option<Color>,
    alpha: f32,

    matrix_stack: Mat3Stack,

    indices_offset: usize,
    batches: SmallVec<BatchInfo, STACK_ALLOCATED_QUADS>,
    vertices: SmallVec<f32, { STACK_ALLOCATED_QUADS * 12 }>,
    indices: SmallVec<u32, { STACK_ALLOCATED_QUADS * 6 }>,

    pub(crate) last_text_bounds: Rect,
    stats: DrawStats,
}

impl Draw2D {
    pub fn new(size: Vec2) -> Self {
        let projection = Mat4::orthographic_rh(0.0, size.x, size.y, 0.0, 0.0, 1.0);
        let inverse_projection = projection.inverse();
        Self {
            size,
            projection,
            inverse_projection,
            alpha: 1.0,
            ..Default::default()
        }
    }

    #[inline]
    pub fn clear(&mut self, color: Color) {
        self.clear_color = Some(color);
    }

    #[inline]
    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    #[inline]
    pub fn set_round_pixels(&mut self, round: bool) {
        self.round_pixels = round;
    }

    #[inline]
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    #[inline]
    pub fn add_element<T>(&mut self, element: &T)
    where
        T: Element2D,
    {
        element.process(self);
        self.stats.elements += 1;
    }

    pub fn add_to_batch<'a>(&'a mut self, info: DrawingInfo<'a>) {
        let start_idx = self.indices.len();
        let end_idx = start_idx + info.indices.len();
        let mut painter = get_mut_2d_painter();
        let PipelineContext {
            pipeline,
            mut groups,
            vertex_offset,
            x_pos,
            y_pos,
            alpha_pos,
        } = painter
            .pipelines
            .get(&info.pipeline)
            .ok_or_else(|| format!("Missing pipeline '{:?}'", info.pipeline))
            .unwrap()
            .clone();

        // assign in spot 1 the texture/sampler binding group
        if let Some(sp) = info.sprite {
            let bind_group = painter.cached_bind_group_for(&pipeline, sp);
            if groups.len() > 1 {
                groups[1] = bind_group;
            } else {
                groups.push(bind_group);
            }
        }

        if matches!(info.pipeline, DrawPipelineId::Text) {
            groups.push(get_mut_text_system().bind_group(&pipeline).clone());
        }

        let vbo_start = self.vertices.len() as u64 * 4;
        let ebo_start = self.indices.len() as u64 * 4;

        let batch = BatchInfo {
            vbo_range: vbo_start..vbo_start,
            ebo_range: ebo_start..ebo_start,
            start_idx,
            end_idx,
            pipeline,
            bind_groups: groups,
        };

        let new_batch = match self.batches.last() {
            None => true,
            Some(last) => !last.is_compatible(&batch),
        };

        if new_batch {
            self.indices_offset = 0;
            self.batches.push(batch);
            self.stats.batches += 1;
        }

        let current = self.batches.last_mut().unwrap();
        current.end_idx = end_idx;

        let vbo_count = info.vertices.len() as u64 * 4; // f32=4bytes
        let ebo_count = info.indices.len() as u64 * 4; // u32=4bytes
        current.vbo_range.end += vbo_count;
        current.ebo_range.end += ebo_count;

        // the indices must use an offset
        self.indices.extend(
            info.indices
                .iter()
                .map(|idx| idx + self.indices_offset as u32),
        );

        self.indices_offset += info.vertices.len() / vertex_offset;

        let matrix = self.matrix() * info.transform;
        info.vertices
            .chunks_exact_mut(vertex_offset)
            .for_each(|chunk| {
                debug_assert!(chunk.len() >= 2);

                let x = chunk[x_pos];
                let y = chunk[y_pos];

                let prev_xyz = if self.round_pixels {
                    vec3(x, y, 1.0).round()
                } else {
                    vec3(x, y, 1.0)
                };

                let xyz = matrix * prev_xyz;
                chunk[x_pos] = xyz.x;
                chunk[y_pos] = xyz.y;

                if let Some(a_pos) = alpha_pos {
                    let alpha = chunk[a_pos] * self.alpha;
                    chunk[a_pos] = alpha;
                }
            });
        self.vertices.extend_from_slice(info.vertices);
    }

    #[inline]
    pub fn last_text_bounds(&self) -> Rect {
        self.last_text_bounds
    }

    // - Transform
    pub fn set_projection(&mut self, projection: Mat4) {
        debug_assert!(
            self.batches.is_empty(),
            "The Draw2D projection must be set before any drawing."
        );
        self.projection = projection;
        self.inverse_projection = self.projection.inverse();
    }

    pub fn set_size(&mut self, size: Vec2) {
        debug_assert!(
            self.batches.is_empty(),
            "The Draw2D size must be set before any drawing."
        );

        if self.size != size {
            self.size = size;
            self.set_projection(Mat4::orthographic_rh(0.0, size.x, size.y, 0.0, 0.0, 1.0));
        }
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn set_camera(&mut self, cam: &dyn BaseCam2D) {
        debug_assert!(
            self.batches.is_empty(),
            "The Camera2D must be set before any drawing."
        );

        self.size = cam.size();

        // projection
        self.projection = cam.projection();
        self.inverse_projection = cam.inverse_projection();

        debug_assert!(
            self.matrix_stack.is_empty(),
            "The Camera2D must be set before push any transformation"
        );

        // transform
        self.matrix_stack.set_matrix(cam.transform());

        // do not assign inverse_transform, it will be calculated and cached when needed
        self.inverse_transform = None;
    }

    #[inline]
    pub fn projection(&self) -> Mat4 {
        self.projection
    }

    pub fn push_matrix(&mut self, m: Mat3) {
        self.matrix_stack.push(m);
        self.inverse_transform = None;
    }

    pub fn set_matrix(&mut self, m: Mat3) {
        self.matrix_stack.set_matrix(m);
        self.inverse_transform = None;
    }

    #[inline]
    pub fn matrix(&self) -> Mat3 {
        self.matrix_stack.matrix()
    }

    pub fn pop_matrix(&mut self) {
        self.matrix_stack.pop();
        self.inverse_transform = None;
    }

    pub fn clear_matrix_stack(&mut self) {
        self.matrix_stack.clear();
        self.inverse_transform = None;
    }

    #[inline]
    pub fn matrix_stack_len(&self) -> usize {
        self.matrix_stack.len()
    }

    /// Translate a local point to screen coordinates
    pub fn local_to_screen(&self, point: Vec2) -> Vec2 {
        let half = self.size * 0.5;
        let pos = self.matrix() * vec3(point.x, point.y, 1.0);
        let pos = self.projection * vec4(pos.x, pos.y, pos.z, 1.0);
        vec2(half.x + (half.x * pos.x), half.y + (half.y * -pos.y))
    }

    /// Translates a screen point to local coordinates
    pub fn screen_to_local(&mut self, point: Vec2) -> Vec2 {
        let transform = self.matrix();
        let inverse_transform = *self
            .inverse_transform
            .get_or_insert_with(|| transform.inverse());

        // normalized coordinates
        debug_assert!(
            self.size.x != 0.0 && self.size.y != 0.0,
            "Draw2D size cannot be 0"
        );
        let norm = point / self.size;
        let pos = norm * vec2(2.0, -2.0) + vec2(-1.0, 1.0);

        // projected position
        let pos = self
            .inverse_projection
            .project_point3(vec3(pos.x, pos.y, 1.0));

        // local position
        inverse_transform.transform_point2(vec2(pos.x, pos.y))
    }

    // - shapes
    #[inline]
    pub fn path(&mut self) -> Drawing<'_, Path2D> {
        Drawing::new(self, Path2D::new())
    }

    #[inline]
    pub fn line(&mut self, p1: Vec2, p2: Vec2) -> Drawing<'_, Line2D> {
        Drawing::new(self, Line2D::new(p1, p2))
    }

    #[inline]
    pub fn triangle(&mut self, p1: Vec2, p2: Vec2, p3: Vec2) -> Drawing<'_, Triangle2D> {
        Drawing::new(self, Triangle2D::new(p1, p2, p3))
    }

    #[inline]
    pub fn rect(&mut self, pos: Vec2, size: Vec2) -> Drawing<'_, Rectangle2D> {
        Drawing::new(self, Rectangle2D::new(pos, size))
    }

    #[inline]
    pub fn circle(&mut self, radius: f32) -> Drawing<'_, Circle2D> {
        Drawing::new(self, Circle2D::new(radius))
    }

    #[inline]
    pub fn ellipse(&mut self, pos: Vec2, size: Vec2) -> Drawing<'_, Ellipse2D> {
        Drawing::new(self, Ellipse2D::new(pos, size))
    }

    #[inline]
    pub fn star(
        &mut self,
        spikes: u8,
        outer_radius: f32,
        inner_radius: f32,
    ) -> Drawing<'_, Star2D> {
        Drawing::new(self, Star2D::new(spikes, outer_radius, inner_radius))
    }

    #[inline]
    pub fn polygon(&mut self, sides: u8, radius: f32) -> Drawing<'_, Polygon2D> {
        Drawing::new(self, Polygon2D::new(sides, radius))
    }

    // - pattern
    #[inline]
    pub fn pattern(&mut self, sprite: &Sprite) -> Drawing<'_, Pattern2D> {
        Drawing::new(self, Pattern2D::new(sprite))
    }

    // - images
    #[inline]
    pub fn image(&mut self, sprite: &Sprite) -> Drawing<'_, Image2D> {
        Drawing::new(self, Image2D::new(sprite))
    }

    // - text
    #[inline]
    pub fn text<'a, 'b: 'a>(&'a mut self, text: &'b str) -> Drawing<'a, Text2D<'a>> {
        Drawing::new(self, Text2D::new(text))
    }

    #[inline]
    pub fn stats(&self) -> DrawStats {
        self.stats
    }

    pub fn clone_transform(&self) -> Self {
        let mut draw = Draw2D::new(self.size);
        draw.set_projection(self.projection);
        draw.matrix_stack = self.matrix_stack.clone();
        draw
    }
}

pub struct DrawingInfo<'a> {
    pub pipeline: DrawPipelineId,
    pub vertices: &'a mut [f32],
    pub indices: &'a [u32],
    pub transform: Mat3,
    pub sprite: Option<&'a Sprite>,
}

pub trait Element2D {
    fn process(&self, draw: &mut Draw2D);
}

impl AsRenderer for Draw2D {
    fn render(&self, target: Option<&RenderTexture>) -> Result<(), String> {
        let painter = get_2d_painter();

        let ubo_transform = &painter.ubo;
        let vbo = &painter.vbo;
        let ebo = &painter.ebo;

        // TODO check dirty transform flag to avoid update all the time this
        gfx::write_buffer(ubo_transform)
            .with_data(self.projection.as_ref())
            .build()
            .unwrap();

        gfx::write_buffer(vbo)
            .with_data(&self.vertices)
            .build()
            .unwrap();

        gfx::write_buffer(ebo)
            .with_data(&self.indices)
            .build()
            .unwrap();

        let mut cleared = false;
        let mut renderer = Renderer::new();

        if self.batches.is_empty() {
            if let Some(color) = self.clear_color {
                renderer.begin_pass().clear_color(color.as_linear());
                cleared = true;
            }
        }

        self.batches.iter().for_each(|b| {
            let pass = renderer.begin_pass();

            // clear only once
            if !cleared {
                if let Some(color) = self.clear_color {
                    pass.clear_color(color.as_linear());
                    cleared = true;
                }
            }

            let binds: ArrayVec<&BindGroup, MAX_BIND_GROUPS_PER_PIPELINE> =
                b.bind_groups.iter().collect();

            pass.pipeline(&b.pipeline)
                .buffers_with_offset(&[(vbo, b.vbo_range.clone()), (ebo, b.ebo_range.clone())])
                .bindings(&binds);

            let count = b.count() as u32;
            pass.draw(0..count);
        });

        self.flush(&renderer, target)
    }
}
