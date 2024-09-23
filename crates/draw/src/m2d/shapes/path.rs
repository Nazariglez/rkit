use crate::shapes::{TessMode, SHAPE_TESSELLATOR};
use crate::{Draw2D, DrawPipeline, DrawingInfo, Element2D};
use core::gfx::Color;
use core::math::{Mat3, Vec2};
use lyon::math::point;
use lyon::path::path::Builder;
use lyon::tessellation::*;
use std::cell::RefCell;

pub struct Path2D {
    stroke_options: StrokeOptions,
    fill_options: FillOptions,
    builder: RefCell<Builder>,
    initialized: bool,
    color: Color,
    alpha: f32,
    transform: Mat3,
    modes: [Option<TessMode>; 2],
    mode_index: usize,
    fill_color: Option<Color>,
    stroke_color: Option<Color>,
}

impl Default for Path2D {
    fn default() -> Self {
        Self::new()
    }
}

impl Path2D {
    pub fn new() -> Self {
        let stroke_options = StrokeOptions::DEFAULT.with_miter_limit(f32::MAX);

        Self {
            stroke_options,
            fill_options: FillOptions::default(),
            builder: RefCell::new(path::Path::builder()),
            initialized: false,
            color: Color::WHITE,
            alpha: 1.0,
            transform: Mat3::IDENTITY,
            modes: [None; 2],
            mode_index: 0,
            fill_color: None,
            stroke_color: None,
        }
    }

    pub fn alpha(&mut self, alpha: f32) -> &mut Self {
        self.alpha = alpha;
        self
    }

    // Start the path on the point given
    pub fn move_to(&mut self, pos: Vec2) -> &mut Self {
        if self.initialized {
            self.builder.borrow_mut().end(false);
        }
        self.builder.borrow_mut().begin(point(pos.x, pos.y));
        self.initialized = true;
        self
    }

    // Draw a line from the previous point to the new point
    pub fn line_to(&mut self, pos: Vec2) -> &mut Self {
        debug_assert!(self.initialized, "You should use move_to first");
        self.builder.borrow_mut().line_to(point(pos.x, pos.y));
        self
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2) -> &mut Self {
        debug_assert!(self.initialized, "You should use move_to first");
        self.builder
            .borrow_mut()
            .quadratic_bezier_to(point(ctrl.x, ctrl.x), point(to.x, to.x));
        self
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) -> &mut Self {
        debug_assert!(self.initialized, "You should use move_to first");
        self.builder.borrow_mut().cubic_bezier_to(
            point(ctrl1.x, ctrl1.y),
            point(ctrl2.x, ctrl2.y),
            point(to.x, to.y),
        );
        self
    }

    // Closes the line drawing a line to the last move_to point
    pub fn close(&mut self) -> &mut Self {
        debug_assert!(self.initialized, "You should use move_to first");
        self.initialized = false;
        self.builder.borrow_mut().end(true);
        self
    }

    pub fn tolerance(&mut self, tolerance: f32) -> &mut Self {
        self.stroke_options = self.stroke_options.with_tolerance(tolerance);
        self.fill_options = self.fill_options.with_tolerance(tolerance);
        self
    }

    pub fn round_cap(&mut self) -> &mut Self {
        self.stroke_options = self
            .stroke_options
            .with_start_cap(LineCap::Round)
            .with_end_cap(LineCap::Round);
        self
    }

    pub fn butt_cap(&mut self) -> &mut Self {
        self.stroke_options = self
            .stroke_options
            .with_start_cap(LineCap::Butt)
            .with_end_cap(LineCap::Butt);
        self
    }

    pub fn square_cap(&mut self) -> &mut Self {
        self.stroke_options = self
            .stroke_options
            .with_start_cap(LineCap::Square)
            .with_end_cap(LineCap::Square);
        self
    }

    pub fn miter_join(&mut self) -> &mut Self {
        self.stroke_options = self.stroke_options.with_line_join(LineJoin::Miter);
        self
    }

    pub fn round_join(&mut self) -> &mut Self {
        self.stroke_options = self.stroke_options.with_line_join(LineJoin::Round);
        self
    }

    pub fn bevel_join(&mut self) -> &mut Self {
        self.stroke_options = self.stroke_options.with_line_join(LineJoin::Bevel);
        self
    }

    pub fn fill(&mut self) -> &mut Self {
        self.modes[self.mode_index] = Some(TessMode::Fill);
        self.mode_index = (self.mode_index + 1) % 2;
        self
    }

    pub fn stroke(&mut self, width: f32) -> &mut Self {
        self.stroke_options = self.stroke_options.with_line_width(width);
        self.modes[self.mode_index] = Some(TessMode::Stroke);
        self.mode_index = (self.mode_index + 1) % 2;
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
}

impl Element2D for Path2D {
    fn process(&self, draw: &mut Draw2D) {
        if self.initialized {
            self.builder.borrow_mut().end(false);
        }

        // default to fill mode
        let first_mode = self.modes[0].unwrap_or(TessMode::Fill);
        match first_mode {
            TessMode::Fill => fill(self, draw),
            TessMode::Stroke => stroke(self, draw),
        }

        if let Some(mode) = self.modes[1] {
            match first_mode {
                TessMode::Fill => fill(self, draw),
                TessMode::Stroke => stroke(self, draw),
            }
        }
    }
}

fn fill(path: &Path2D, draw: &mut Draw2D) {
    let color = path.fill_color.unwrap_or(path.color);
    let color = color.with_alpha(color.a * path.alpha);

    let raw = path.builder.borrow().clone().build();
    let (mut vertices, indices) = SHAPE_TESSELLATOR.with(|st| {
        st.borrow_mut()
            .fill_lyon_path(&raw, color, &path.fill_options)
    });

    draw.add_to_batch(DrawingInfo {
        pipeline: DrawPipeline::Shapes,
        vertices: &mut vertices,
        indices: &indices,
        transform: path.transform,
        sprite: None,
    });
}

fn stroke(path: &Path2D, draw: &mut Draw2D) {
    let color = path.stroke_color.unwrap_or(path.color);
    let color = color.with_alpha(color.a * path.alpha);

    let raw = path.builder.borrow().clone().build();
    let (mut vertices, indices) = SHAPE_TESSELLATOR.with(|st| {
        st.borrow_mut()
            .stroke_lyon_path(&raw, color, &path.stroke_options)
    });

    draw.add_to_batch(DrawingInfo {
        pipeline: DrawPipeline::Shapes,
        vertices: &mut vertices,
        indices: &indices,
        transform: path.transform,
        sprite: None,
    });
}