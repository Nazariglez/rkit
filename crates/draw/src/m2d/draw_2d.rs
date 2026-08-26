use crate::{
    BaseCam2D, Circle2D, Ellipse2D, Pattern2D, Polygon2D, Star2D, get_mut_2d_painter,
    m2d::{
        clip::{
            ClipFrame, ClipKind, Coverage, GeometryRange, MaskState, RoundedGeometry, project_rect,
            rounded_geometry,
        },
        images::Image2D,
        mat3_stack::Mat3Stack,
        nine_slice::NineSlice2D,
        painter::{ContentPipeline, DrawPipelineId},
        shapes::{Line2D, Path2D, Rectangle2D, Triangle2D},
        text::{RichText2D, Text2D},
    },
    sprite::Sprite,
    text::{RichTextLayout, get_mut_text_system},
};
use arrayvec::ArrayVec;
use corelib::{
    gfx::{
        self, AsRenderer, BindGroup, BindGroupId, Buffer, Color, PipelineId, RenderCommand,
        RenderPass, RenderPipeline, RenderTexture, Renderer, consts::MAX_BIND_GROUPS_PER_PIPELINE,
    },
    math::{Mat3, Mat4, Rect, Vec2, orthographic, vec2, vec3, vec4},
};
use smallvec::SmallVec;
use std::{
    ops::{Deref, DerefMut, Range},
    sync::Arc,
};

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
            "Bind Groups must be less than {MAX_BIND_GROUPS_PER_PIPELINE}"
        );
        (self as &[_]).try_into().unwrap()
    }
}

#[derive(Clone)]
enum DrawOp {
    Batch(BatchInfo),
    MaskPush(MaskState),
    MaskPop(MaskState),
}

#[derive(Clone, PartialEq)]
struct BatchKey {
    pipeline: PipelineId,
    bind_groups: ArrayVec<BindGroupId, MAX_BIND_GROUPS_PER_PIPELINE>,
    coverage: Coverage,
    stencil_reference: Option<u8>,
}

#[derive(Clone)]
struct BatchInfo {
    key: BatchKey,
    vbo_range: Range<u64>,
    ebo_range: Range<u64>,
    start_idx: usize,
    end_idx: usize,
    content: Arc<ContentPipeline>,
    bind_groups: ArrayVec<BindGroup, MAX_BIND_GROUPS_PER_PIPELINE>,
}

impl BatchInfo {
    fn pipeline(&self, clipped: bool) -> Result<&RenderPipeline, String> {
        if clipped {
            self.content.clipped()
        } else {
            Ok(self.content.base())
        }
    }

    fn count(&self) -> Result<u32, String> {
        u32::try_from(self.end_idx - self.start_idx)
            .map_err(|_| "Draw batch index count exceeds the supported range".to_string())
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

    indices_offset: u32,
    operations: SmallVec<DrawOp, STACK_ALLOCATED_QUADS>,
    clip_stack: SmallVec<ClipFrame, 8>,
    recording_error: Option<String>,
    recording_started: bool,
    uses_rounded_clip: bool,
    has_stencil_content: bool,
    vertices: SmallVec<f32, { STACK_ALLOCATED_QUADS * 12 }>,
    indices: SmallVec<u32, { STACK_ALLOCATED_QUADS * 6 }>,

    pub(crate) last_text_bounds: Rect,
    stats: DrawStats,
}

impl Draw2D {
    pub fn new(size: Vec2) -> Self {
        let projection = orthographic(0.0, size.x, size.y, 0.0, 0.0, 1.0);
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

    pub fn push_clip(&mut self, rect: Rect) {
        if self.recording_error.is_some() {
            return;
        }
        let coverage = match project_rect(rect, self.matrix(), self.projection) {
            Ok(coverage) => self.current_coverage().intersect(coverage),
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        self.recording_started = true;
        self.clip_stack.push(ClipFrame {
            coverage,
            stencil_depth: self.current_stencil_depth(),
            kind: ClipKind::Rect,
        });
    }

    pub fn push_rounded_clip(&mut self, rect: Rect, radius: f32) {
        if self.recording_error.is_some() {
            return;
        }
        if self.has_stencil_content {
            self.record_error(
                "Draw2D rounded clips cannot be combined with a stencil-owning pipeline",
            );
            return;
        }

        let parent_coverage = self.current_coverage();
        let parent_depth = self.current_stencil_depth();
        let geometry = match rounded_geometry(rect, radius, self.matrix(), self.projection) {
            Ok(geometry) => geometry,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        self.recording_started = true;

        let Some(RoundedGeometry {
            vertices,
            indices,
            bounds,
        }) = geometry
        else {
            self.clip_stack.push(ClipFrame {
                coverage: Coverage::Empty,
                stencil_depth: parent_depth,
                kind: ClipKind::Rounded(None),
            });
            return;
        };
        let coverage = parent_coverage.intersect(Coverage::Rect(bounds));
        let Coverage::Rect(coverage_rect) = coverage else {
            self.clip_stack.push(ClipFrame {
                coverage: Coverage::Empty,
                stencil_depth: parent_depth,
                kind: ClipKind::Rounded(None),
            });
            return;
        };
        let Some(child_depth) = parent_depth.checked_add(1) else {
            self.record_error("Draw2D rounded clip nesting exceeds the stencil depth limit");
            return;
        };
        if let Err(error) = self.enable_rounded_clipping() {
            self.record_error(error);
            return;
        }
        let Ok(count) = u32::try_from(indices.len()) else {
            self.record_error("Rounded clip index count exceeds the supported range");
            return;
        };

        let vbo_start = self.vertices.len() as u64 * 4;
        let ebo_start = self.indices.len() as u64 * 4;
        let state = MaskState {
            geometry: GeometryRange {
                vbo: vbo_start..vbo_start + vertices.len() as u64 * 4,
                ebo: ebo_start..ebo_start + indices.len() as u64 * 4,
                count,
            },
            coverage: coverage_rect,
            parent_depth,
            child_depth,
        };
        self.vertices.extend_from_slice(&vertices);
        self.indices.extend_from_slice(&indices);
        self.indices_offset = 0;
        self.operations.push(DrawOp::MaskPush(state.clone()));
        self.clip_stack.push(ClipFrame {
            coverage,
            stencil_depth: child_depth,
            kind: ClipKind::Rounded(Some(state)),
        });
    }

    pub fn pop_clip(&mut self) {
        if self.recording_error.is_some() {
            return;
        }
        let Some(frame) = self.clip_stack.pop() else {
            self.record_error("Draw2D clip stack underflow");
            return;
        };
        self.recording_started = true;
        if let ClipKind::Rounded(Some(mask)) = frame.kind {
            self.operations.push(DrawOp::MaskPop(mask));
            self.indices_offset = 0;
        }
    }

    fn current_coverage(&self) -> Coverage {
        self.clip_stack
            .last()
            .map_or(Coverage::Full, |frame| frame.coverage)
    }

    fn current_stencil_depth(&self) -> u8 {
        self.clip_stack
            .last()
            .map_or(0, |frame| frame.stencil_depth)
    }

    fn record_error(&mut self, error: impl Into<String>) {
        if self.recording_error.is_none() {
            self.recording_error = Some(error.into());
        }
    }

    fn enable_rounded_clipping(&mut self) -> Result<(), String> {
        if self.uses_rounded_clip {
            return Ok(());
        }
        for operation in &self.operations {
            if let DrawOp::Batch(batch) = operation {
                batch.content.clipped()?;
            }
        }
        self.uses_rounded_clip = true;
        Ok(())
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
        if self.recording_error.is_some() {
            return;
        }
        self.recording_started = true;

        let coverage = self.current_coverage();
        if coverage == Coverage::Empty {
            return;
        }
        let stencil_depth = self.current_stencil_depth();

        let mut painter = get_mut_2d_painter();
        let resolved = match painter.resolve_pipeline(info.pipeline, info.sprite) {
            Ok(resolved) => resolved,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        if self.uses_rounded_clip
            && let Err(error) = resolved.content.clipped()
        {
            self.record_error(error);
            return;
        }
        if resolved.uses_stencil {
            self.has_stencil_content = true;
            if self.uses_rounded_clip {
                self.record_error(
                    "Draw2D rounded clips cannot be combined with a stencil-owning pipeline",
                );
                return;
            }
        }

        let text_group = matches!(info.pipeline, DrawPipelineId::Text).then(|| {
            get_mut_text_system()
                .bind_group(resolved.content.base())
                .clone()
        });
        let mut bind_groups = resolved
            .groups
            .iter()
            .map(|group| group.id())
            .collect::<ArrayVec<_, MAX_BIND_GROUPS_PER_PIPELINE>>();
        if let Some(group) = &text_group {
            bind_groups.push(group.id());
        }
        let key = BatchKey {
            pipeline: resolved.content.base().id(),
            bind_groups,
            coverage,
            stencil_reference: self.uses_rounded_clip.then_some(stencil_depth),
        };
        let new_batch = match self.operations.last() {
            Some(DrawOp::Batch(last)) => last.key != key,
            _ => true,
        };
        let batch_resources = new_batch.then(|| {
            let mut groups = resolved
                .groups
                .iter()
                .map(|group| (*group).clone())
                .collect::<ArrayVec<_, MAX_BIND_GROUPS_PER_PIPELINE>>();
            if let Some(group) = text_group {
                groups.push(group);
            }
            (Arc::clone(resolved.content), groups)
        });
        let vertex_offset = resolved.vertex_offset;
        let x_pos = resolved.x_pos;
        let y_pos = resolved.y_pos;
        let alpha_pos = resolved.alpha_pos;
        drop(resolved);
        drop(painter);

        let indices_offset = if new_batch { 0 } else { self.indices_offset };
        let vertex_count = info.vertices.len() / vertex_offset;
        let Ok(vertex_count) = u32::try_from(vertex_count) else {
            self.record_error("Draw batch vertex count exceeds the supported range");
            return;
        };
        let Some(next_indices_offset) = indices_offset.checked_add(vertex_count) else {
            self.record_error("Draw batch vertex offset exceeds the supported range");
            return;
        };
        let index_overflow = info
            .indices
            .iter()
            .any(|index| index.checked_add(indices_offset).is_none());
        if index_overflow {
            self.record_error("Draw batch index exceeds the supported range");
            return;
        }

        let start_idx = self.indices.len();
        let end_idx = start_idx + info.indices.len();
        if let Some((content, bind_groups)) = batch_resources {
            self.indices_offset = 0;
            self.operations.push(DrawOp::Batch(BatchInfo {
                key,
                vbo_range: self.vertices.len() as u64 * 4..self.vertices.len() as u64 * 4,
                ebo_range: self.indices.len() as u64 * 4..self.indices.len() as u64 * 4,
                start_idx,
                end_idx,
                content,
                bind_groups,
            }));
            self.stats.batches += 1;
        }

        let DrawOp::Batch(current) = self.operations.last_mut().unwrap() else {
            unreachable!();
        };
        current.end_idx = end_idx;
        current.vbo_range.end += info.vertices.len() as u64 * 4;
        current.ebo_range.end += info.indices.len() as u64 * 4;

        self.indices
            .extend(info.indices.iter().map(|index| index + indices_offset));
        self.indices_offset = next_indices_offset;

        let matrix = self.matrix() * info.transform;
        for vertex in info.vertices.chunks_exact_mut(vertex_offset) {
            let position = matrix * vec3(vertex[x_pos], vertex[y_pos], 1.0);
            let (x, y) = if self.round_pixels {
                (position.x.round(), position.y.round())
            } else {
                (position.x, position.y)
            };
            vertex[x_pos] = x;
            vertex[y_pos] = y;

            if let Some(alpha_pos) = alpha_pos {
                vertex[alpha_pos] *= self.alpha;
            }
        }
        self.vertices.extend_from_slice(info.vertices);
    }

    #[inline]
    pub fn last_text_bounds(&self) -> Rect {
        self.last_text_bounds
    }

    // - Transform
    #[inline]
    pub fn set_projection(&mut self, projection: Mat4) {
        if self.recording_started {
            self.record_error("Draw2D projection cannot change after recording has started");
            return;
        }
        self.projection = projection;
        self.inverse_projection = self.projection.inverse();
    }

    #[inline]
    pub fn set_size(&mut self, size: Vec2) {
        if self.recording_started {
            self.record_error("Draw2D size cannot change after recording has started");
            return;
        }
        if self.size != size {
            self.size = size;
            self.set_projection(orthographic(0.0, size.x, size.y, 0.0, 0.0, 1.0));
        }
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn set_camera(&mut self, cam: &dyn BaseCam2D) {
        if self.recording_started {
            self.record_error("Draw2D camera cannot change after recording has started");
            return;
        }
        self.size = cam.size();
        self.projection = cam.projection();
        self.inverse_projection = cam.inverse_projection();
        self.matrix_stack.set_matrix(cam.transform());
        self.inverse_transform = None;
    }

    #[inline]
    pub fn projection(&self) -> Mat4 {
        self.projection
    }

    #[inline]
    pub fn push_matrix(&mut self, m: Mat3) {
        self.matrix_stack.push(m);
        self.inverse_transform = None;
    }

    #[inline]
    pub fn set_matrix(&mut self, m: Mat3) {
        self.matrix_stack.set_matrix(m);
        self.inverse_transform = None;
    }

    #[inline]
    pub fn matrix(&self) -> Mat3 {
        self.matrix_stack.matrix()
    }

    #[inline]
    pub fn pop_matrix(&mut self) {
        self.matrix_stack.pop();
        self.inverse_transform = None;
    }

    #[inline]
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

    // - nine slice
    #[inline]
    pub fn nine_slice(&mut self, sprite: &Sprite) -> Drawing<'_, NineSlice2D> {
        Drawing::new(self, NineSlice2D::new(sprite))
    }

    // - text
    #[inline]
    pub fn text<'a, 'b: 'a>(&'a mut self, text: &'b str) -> Drawing<'a, Text2D<'a>> {
        Drawing::new(self, Text2D::new(text))
    }

    #[inline]
    pub fn rich_text<'a>(&'a mut self, layout: &'a RichTextLayout) -> Drawing<'a, RichText2D<'a>> {
        Drawing::new(self, RichText2D::new(layout))
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
        if let Some(error) = &self.recording_error {
            return Err(error.clone());
        }
        if self.uses_rounded_clip
            && let Some(target) = target
            && !target.has_depth()
        {
            return Err(
                "Draw2D rounded clips require a render texture created with with_depth(true)"
                    .to_string(),
            );
        }
        if self.operations.is_empty() {
            let Some(color) = self.clear_color else {
                return Ok(());
            };
            let mut renderer = Renderer::new();
            renderer.begin_pass().clear_color(color.as_linear());
            return self.flush(&renderer, target);
        }

        let has_masks = self
            .operations
            .iter()
            .any(|operation| !matches!(operation, DrawOp::Batch(_)));
        let painter = get_mut_2d_painter();
        let ubo_transform = painter.ubo.clone();
        let vbo = painter.vbo.clone();
        let ebo = painter.ebo.clone();
        let mask_pipelines = if has_masks {
            let (push, pop) = painter.mask_pipelines()?;
            Some((push.clone(), pop.clone()))
        } else {
            None
        };
        drop(painter);

        gfx::write_buffer(&ubo_transform)
            .with_data(self.projection.as_ref())
            .build()?;
        gfx::write_buffer(&vbo).with_data(&self.vertices).build()?;
        gfx::write_buffer(&ebo).with_data(&self.indices).build()?;

        let mut renderer = Renderer::new();
        let mut pass = RenderPass::new();
        if let Some(color) = self.clear_color {
            pass.clear_color(color.as_linear());
        }
        if has_masks {
            pass.clear_stencil(0);
        }

        let mut pass_uses_depth_stencil = None;
        for operation in &self.operations {
            let operation_uses_depth_stencil = match operation {
                DrawOp::Batch(batch) => {
                    batch.pipeline(self.uses_rounded_clip)?.uses_depth_stencil()
                }
                DrawOp::MaskPush(_) | DrawOp::MaskPop(_) => true,
            };
            if pass_uses_depth_stencil
                .is_some_and(|current| current != operation_uses_depth_stencil)
            {
                renderer.add_pass(pass);
                pass = RenderPass::new();
            }
            pass_uses_depth_stencil = Some(operation_uses_depth_stencil);
            append_operation(
                &mut pass,
                operation,
                &vbo,
                &ebo,
                mask_pipelines.as_ref(),
                self.uses_rounded_clip,
            )?;
        }

        if let Some((_, pop_pipeline)) = mask_pipelines.as_ref() {
            for frame in self.clip_stack.iter().rev() {
                let ClipKind::Rounded(Some(mask)) = &frame.kind else {
                    continue;
                };
                let command = pass.begin_command();
                command
                    .pipeline(pop_pipeline)
                    .buffers_with_offset(&[
                        (&vbo, mask.geometry.vbo.clone()),
                        (&ebo, mask.geometry.ebo.clone()),
                    ])
                    .stencil_reference(mask.child_depth)
                    .draw(0..mask.geometry.count);
                apply_coverage(command, Coverage::Rect(mask.coverage));
            }
        }
        renderer.add_pass(pass);

        self.flush(&renderer, target)
    }
}

fn append_operation<'a>(
    pass: &mut RenderPass<'a>,
    operation: &'a DrawOp,
    vbo: &'a Buffer,
    ebo: &'a Buffer,
    mask_pipelines: Option<&'a (RenderPipeline, RenderPipeline)>,
    clipped: bool,
) -> Result<(), String> {
    let command = pass.begin_command();
    match operation {
        DrawOp::Batch(batch) => {
            let bindings: ArrayVec<&BindGroup, MAX_BIND_GROUPS_PER_PIPELINE> =
                batch.bind_groups.iter().collect();
            command
                .pipeline(batch.pipeline(clipped)?)
                .buffers_with_offset(&[
                    (vbo, batch.vbo_range.clone()),
                    (ebo, batch.ebo_range.clone()),
                ])
                .bindings(&bindings);
            apply_coverage(command, batch.key.coverage);
            if let Some(reference) = batch.key.stencil_reference {
                command.stencil_reference(reference);
            }
            command.draw(0..batch.count()?);
        }
        DrawOp::MaskPush(mask) => {
            let (push_pipeline, _) = mask_pipelines.unwrap();
            command
                .pipeline(push_pipeline)
                .buffers_with_offset(&[
                    (vbo, mask.geometry.vbo.clone()),
                    (ebo, mask.geometry.ebo.clone()),
                ])
                .stencil_reference(mask.parent_depth)
                .draw(0..mask.geometry.count);
            apply_coverage(command, Coverage::Rect(mask.coverage));
        }
        DrawOp::MaskPop(mask) => {
            let (_, pop_pipeline) = mask_pipelines.unwrap();
            command
                .pipeline(pop_pipeline)
                .buffers_with_offset(&[
                    (vbo, mask.geometry.vbo.clone()),
                    (ebo, mask.geometry.ebo.clone()),
                ])
                .stencil_reference(mask.child_depth)
                .draw(0..mask.geometry.count);
            apply_coverage(command, Coverage::Rect(mask.coverage));
        }
    }
    Ok(())
}

fn apply_coverage(command: &mut RenderCommand<'_>, coverage: Coverage) {
    if let Coverage::Rect(rect) = coverage {
        command.normalized_scissors(rect.min.x, rect.min.y, rect.max.x, rect.max.y);
    }
}
