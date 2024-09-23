use crate::shapes::{TessMode, SHAPE_TESSELLATOR};
use crate::{Draw2D, DrawPipeline, DrawingInfo, Element2D, Triangle2D};
use core::gfx::Color;
use core::math::{Mat3, Vec2};
use lyon::math::{point, vector, Angle};
use lyon::path::{Path, Winding};
use lyon::tessellation::*;

pub struct Ellipse2D {
    color: Color,
    pos: Vec2,
    size: Vec2,
    rotation: f32,
    stroke_width: f32,
    alpha: f32,
    transform: Mat3,
    tolerance: f32,
    modes: [Option<TessMode>; 2],
    mode_index: usize,
    fill_color: Option<Color>,
    stroke_color: Option<Color>,
}

impl Ellipse2D {
    pub fn new(pos: Vec2, size: Vec2) -> Self {
        Self {
            color: Color::WHITE,
            pos,
            size,
            rotation: 0.0,
            stroke_width: 1.0,
            alpha: 1.0,
            transform: Mat3::IDENTITY,
            tolerance: StrokeOptions::DEFAULT_TOLERANCE,
            modes: [None; 2],
            mode_index: 0,
            fill_color: None,
            stroke_color: None,
        }
    }

    pub fn rotate(&mut self, radians: f32) -> &mut Self {
        self.rotation = radians;
        self
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

impl Element2D for Ellipse2D {
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

fn stroke(ellipse: &Ellipse2D, draw: &mut Draw2D) {
    let stroke_options = StrokeOptions::default()
        .with_line_width(ellipse.stroke_width)
        .with_tolerance(ellipse.tolerance);

    let color = ellipse.stroke_color.unwrap_or(ellipse.color);
    let color = color.with_alpha(color.a * ellipse.alpha);

    let raw = tess_ellipse(
        ellipse.pos.x,
        ellipse.pos.y,
        ellipse.size.x,
        ellipse.size.y,
        ellipse.rotation,
    );
    let (mut vertices, indices) = SHAPE_TESSELLATOR.with(|st| {
        st.borrow_mut()
            .stroke_lyon_path(&raw, color, &stroke_options)
    });

    draw.add_to_batch(DrawingInfo {
        pipeline: DrawPipeline::Shapes,
        vertices: &mut vertices,
        indices: &indices,
        transform: ellipse.transform,
        sprite: None,
    });
}

fn fill(ellipse: &Ellipse2D, draw: &mut Draw2D) {
    let fill_options = FillOptions::default().with_tolerance(ellipse.tolerance);

    let color = ellipse.fill_color.unwrap_or(ellipse.color);
    let color = color.with_alpha(color.a * ellipse.alpha);

    let raw = tess_ellipse(
        ellipse.pos.x,
        ellipse.pos.y,
        ellipse.size.x,
        ellipse.size.y,
        ellipse.rotation,
    );
    let (mut vertices, indices) =
        SHAPE_TESSELLATOR.with(|st| st.borrow_mut().fill_lyon_path(&raw, color, &fill_options));

    draw.add_to_batch(DrawingInfo {
        pipeline: DrawPipeline::Shapes,
        vertices: &mut vertices,
        indices: &indices,
        transform: ellipse.transform,
        sprite: None,
    });
}

pub fn tess_ellipse(x: f32, y: f32, width: f32, height: f32, rotation: f32) -> Path {
    let mut builder = Path::builder();
    builder.add_ellipse(
        point(x, y),
        vector(width, height),
        Angle::radians(rotation),
        Winding::Positive,
    );
    builder.build()
}