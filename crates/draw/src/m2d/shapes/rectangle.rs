use crate::shapes::{TessMode, SHAPE_TESSELLATOR};
use crate::{Draw2D, DrawPipelineId, DrawingInfo, Element2D, Transform2D};
use core::gfx::Color;
use core::math::{bvec2, Mat3, Vec2};
use lyon::math::{point, Box2D};
use lyon::path::builder::BorderRadii;
use lyon::path::{Path, Winding};
use lyon::tessellation::*;
use macros::Drawable2D;

#[derive(Drawable2D)]
pub struct Rectangle2D {
    color: Color,
    pos: Vec2,
    size: Vec2,
    stroke_width: f32,
    alpha: f32,
    rounded_corners: Option<[f32; 4]>,
    corner_tolerance: f32,
    modes: [Option<TessMode>; 2],
    mode_index: usize,
    fill_color: Option<Color>,
    stroke_color: Option<Color>,

    #[pipeline_id]
    pip: DrawPipelineId,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl Rectangle2D {
    pub fn new(position: Vec2, size: Vec2) -> Self {
        Self {
            color: Color::WHITE,
            pos: position,
            size,
            stroke_width: 1.0,
            alpha: 1.0,
            rounded_corners: None,
            corner_tolerance: FillOptions::DEFAULT_TOLERANCE,
            modes: [None; 2],
            mode_index: 0,
            fill_color: None,
            stroke_color: None,

            pip: DrawPipelineId::Shapes,
            transform: None,
        }
    }

    pub fn corner_radius(&mut self, radius: f32) -> &mut Self {
        self.rounded_corners = Some([radius; 4]);
        self
    }

    pub fn corner_tolerance(&mut self, tolerance: f32) -> &mut Self {
        self.corner_tolerance = tolerance;
        self
    }

    pub fn top_left_radius(&mut self, radius: f32) -> &mut Self {
        let mut corners = self.rounded_corners.unwrap_or([0.0, 0.0, 0.0, 0.0]);
        corners[0] = radius;
        self
    }

    pub fn top_right_radius(&mut self, radius: f32) -> &mut Self {
        let mut corners = self.rounded_corners.unwrap_or([0.0, 0.0, 0.0, 0.0]);
        corners[1] = radius;
        self
    }

    pub fn bottom_left_radius(&mut self, radius: f32) -> &mut Self {
        let mut corners = self.rounded_corners.unwrap_or([0.0, 0.0, 0.0, 0.0]);
        corners[2] = radius;
        self
    }

    pub fn bottom_right_radius(&mut self, radius: f32) -> &mut Self {
        let mut corners = self.rounded_corners.unwrap_or([0.0, 0.0, 0.0, 0.0]);
        corners[3] = radius;
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

impl Element2D for Rectangle2D {
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

fn stroke(quad: &Rectangle2D, draw: &mut Draw2D) {
    let stroke_options = StrokeOptions::default()
        .with_miter_limit(quad.stroke_width * 2.0)
        .with_line_width(quad.stroke_width)
        .with_tolerance(quad.corner_tolerance);

    let color = quad.stroke_color.unwrap_or(quad.color);
    let color = color.with_alpha(color.a * quad.alpha);

    let Vec2 { x, y } = quad.pos;
    let Vec2 {
        x: width,
        y: height,
    } = quad.size;

    let raw = match quad.rounded_corners {
        Some([tl, tr, bl, br]) => rounded_rect(x, y, width, height, (tl, tr, bl, br)),
        _ => rectangle(x, y, width, height),
    };

    let (mut vertices, indices) = SHAPE_TESSELLATOR.with(|st| {
        st.borrow_mut()
            .stroke_lyon_path(&raw, color, &stroke_options)
    });

    let matrix = quad
        .transform
        .map_or(Mat3::IDENTITY, |mut t| t.set_size(quad.size).updated_mat3());

    draw.add_to_batch(DrawingInfo {
        pipeline: quad.pip,
        vertices: &mut vertices,
        indices: &indices,
        transform: matrix,
        sprite: None,
    });
}

fn fill(quad: &Rectangle2D, draw: &mut Draw2D) {
    let mut draw_shape = |mut vertices: &mut [f32], indices: &[u32]| {
        let matrix = quad
            .transform
            .map_or(Mat3::IDENTITY, |mut t| t.set_size(quad.size).updated_mat3());

        draw.add_to_batch(DrawingInfo {
            pipeline: quad.pip,
            vertices: &mut vertices,
            indices: &indices,
            transform: matrix,
            sprite: None,
        });
    };

    let c = quad.fill_color.unwrap_or(quad.color);
    let c = c.with_alpha(c.a * quad.alpha);

    let Vec2 { x: x1, y: y1 } = quad.pos;
    let Vec2 {
        x: width,
        y: height,
    } = quad.size;

    match quad.rounded_corners {
        Some([tl, tr, bl, br]) => {
            let raw = rounded_rect(x1, y1, width, height, (tl, tr, bl, br));
            let options = FillOptions::default().with_tolerance(quad.corner_tolerance);
            let (mut vertices, indices) =
                SHAPE_TESSELLATOR.with(|st| st.borrow_mut().fill_lyon_path(&raw, c, &options));

            draw_shape(&mut vertices, &indices);
        }
        _ => {
            let x2 = x1 + width;
            let y2 = y1 + height;

            let indices = [0, 1, 2, 0, 2, 3];

            #[rustfmt::skip]
            let mut vertices = [
                x1, y1, c.r, c.g, c.b, c.a,
                x1, y2, c.r, c.g, c.b, c.a,
                x2, y2, c.r, c.g, c.b, c.a,
                x2, y1, c.r, c.g, c.b, c.a,
            ];

            draw_shape(&mut vertices, &indices);
        }
    };
}

fn rectangle(x: f32, y: f32, width: f32, height: f32) -> Path {
    let mut builder = Path::builder();
    builder.add_rectangle(
        &Box2D {
            min: point(x, y),
            max: point(x + width, y + height),
        },
        Winding::Positive,
    );
    builder.build()
}

fn rounded_rect(x: f32, y: f32, width: f32, height: f32, corner: (f32, f32, f32, f32)) -> Path {
    let (tl, tr, bl, br) = corner;

    let mut builder = Path::builder();
    builder.add_rounded_rectangle(
        &Box2D {
            min: point(x, y),
            max: point(x + width, y + height),
        },
        &BorderRadii {
            top_left: tl,
            top_right: tr,
            bottom_left: bl,
            bottom_right: br,
        },
        Winding::Positive,
    );
    builder.build()
}
