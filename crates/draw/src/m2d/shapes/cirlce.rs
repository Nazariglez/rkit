use crate::shapes::{TessMode, SHAPE_TESSELLATOR};
use crate::{Draw2D, DrawPipeline, DrawingInfo, Element2D, Transform2D, Triangle2D};
use core::gfx::Color;
use core::math::{bvec2, Mat3, Vec2};
use lyon::math::point;
use lyon::path::{Path, Winding};
use lyon::tessellation::*;
use macros::Transform2D;

#[derive(Transform2D)]
pub struct Circle2D {
    color: Color,
    pos: Vec2,
    radius: f32,
    stroke_width: f32,
    alpha: f32,
    tolerance: f32,
    modes: [Option<TessMode>; 2],
    mode_index: usize,
    fill_color: Option<Color>,
    stroke_color: Option<Color>,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl Circle2D {
    pub fn new(radius: f32) -> Self {
        Self {
            color: Color::WHITE,
            pos: Vec2::splat(0.0),
            radius,
            stroke_width: 1.0,
            alpha: 1.0,
            tolerance: StrokeOptions::DEFAULT_TOLERANCE,
            modes: [None; 2],
            mode_index: 0,
            fill_color: None,
            stroke_color: None,

            transform: None,
        }
    }

    pub fn position(&mut self, pos: Vec2) -> &mut Self {
        self.pos = pos;
        self
    }

    pub fn tolerance(&mut self, value: f32) -> &mut Self {
        self.tolerance = value;
        self
    }

    pub fn fill_color(&mut self, color: Color) -> &mut Self {
        self.fill_color = Some(color);
        self
    }

    pub fn stroke_color(&mut self, color: Color) -> &mut Self {
        self.stroke_color = Some(color);
        self
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }

    pub fn alpha(&mut self, alpha: f32) -> &mut Self {
        self.alpha = alpha;
        self
    }

    pub fn fill(&mut self) -> &mut Self {
        self.modes[self.mode_index] = Some(TessMode::Fill);
        self.mode_index = (self.mode_index + 1) % 2;
        self
    }

    pub fn stroke(&mut self, width: f32) -> &mut Self {
        self.modes[self.mode_index] = Some(TessMode::Stroke);
        self.stroke_width = width;
        self.mode_index = (self.mode_index + 1) % 2;
        self
    }
}

impl Element2D for Circle2D {
    fn process(&self, draw: &mut Draw2D) {
        // default to fill mode
        let first_mode = self.modes[0].unwrap_or(TessMode::Fill);
        match first_mode {
            TessMode::Fill => fill(self, draw),
            TessMode::Stroke => stroke(self, draw),
        }

        if let Some(mode) = self.modes[1] {
            match mode {
                TessMode::Fill => fill(self, draw),
                TessMode::Stroke => stroke(self, draw),
            }
        }
    }
}

fn stroke(circle: &Circle2D, draw: &mut Draw2D) {
    let stroke_options = StrokeOptions::default()
        .with_line_width(circle.stroke_width)
        .with_tolerance(circle.tolerance);

    let color = circle.stroke_color.unwrap_or(circle.color);
    let color = color.with_alpha(color.a * circle.alpha);

    let size = Vec2::splat(circle.radius * 2.0);
    let pos = circle.pos + size * 0.5;

    let raw = tess_circle(pos.x, pos.y, circle.radius);
    let (mut vertices, indices) = SHAPE_TESSELLATOR.with(|st| {
        st.borrow_mut()
            .stroke_lyon_path(&raw, color, &stroke_options)
    });

    let matrix = circle
        .transform
        .map_or(Mat3::IDENTITY, |mut t| t.set_size(size).as_mat3());

    draw.add_to_batch(DrawingInfo {
        pipeline: DrawPipeline::Shapes,
        vertices: &mut vertices,
        indices: &indices,
        transform: matrix,
        sprite: None,
    });
}

fn fill(circle: &Circle2D, draw: &mut Draw2D) {
    let fill_options = FillOptions::default().with_tolerance(circle.tolerance);

    let color = circle.fill_color.unwrap_or(circle.color);
    let color = color.with_alpha(color.a * circle.alpha);

    let size = Vec2::splat(circle.radius * 2.0);
    let pos = circle.pos + size * 0.5;

    let raw = tess_circle(pos.x, pos.y, circle.radius);
    let (mut vertices, indices) =
        SHAPE_TESSELLATOR.with(|st| st.borrow_mut().fill_lyon_path(&raw, color, &fill_options));

    let matrix = circle
        .transform
        .map_or(Mat3::IDENTITY, |mut t| t.set_size(size).as_mat3());

    draw.add_to_batch(DrawingInfo {
        pipeline: DrawPipeline::Shapes,
        vertices: &mut vertices,
        indices: &indices,
        transform: matrix,
        sprite: None,
    });
}

fn tess_circle(x: f32, y: f32, radius: f32) -> Path {
    let mut builder = Path::builder();
    builder.add_circle(point(x, y), radius, Winding::Positive);
    builder.build()
}
