use crate::m2d::shapes::Path2D;
use crate::shapes::TessMode;
use crate::{Draw2D, DrawPipeline, DrawingInfo, Element2D};
use core::gfx::Color;
use core::math::{Mat3, Vec2};

pub struct Triangle2D {
    points: [Vec2; 3],
    color: Color,
    alpha: f32,
    transform: Mat3,
    stroke_width: f32,
    modes: [Option<TessMode>; 2],
    mode_index: usize,
    fill_color: Option<Color>,
    stroke_color: Option<Color>,
}

impl Triangle2D {
    pub fn new(p1: Vec2, p2: Vec2, p3: Vec2) -> Self {
        Self {
            points: [p1, p2, p3],
            color: Color::WHITE,
            alpha: 1.0,
            transform: Mat3::IDENTITY,
            stroke_width: 1.0,
            modes: [None; 2],
            mode_index: 0,
            fill_color: None,
            stroke_color: None,
        }
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

impl Element2D for Triangle2D {
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

fn fill(triangle: &Triangle2D, draw: &mut Draw2D) {
    let [a, b, c] = triangle.points;
    let color = triangle.fill_color.unwrap_or(triangle.color);
    let alpha = triangle.alpha;

    #[rustfmt::skip]
    let mut vertices = [
        a.x, a.y, color.r, color.g, color.b, color.a * alpha,
        b.x, b.y, color.r, color.g, color.b, color.a * alpha,
        c.x, c.y, color.r, color.g, color.b, color.a * alpha,
    ];

    let indices = [0, 1, 2];

    draw.add_to_batch(DrawingInfo {
        pipeline: DrawPipeline::Shapes,
        vertices: &mut vertices,
        indices: &indices,
        transform: triangle.transform,
        sprite: None,
    });
}

fn stroke(triangle: &Triangle2D, draw: &mut Draw2D) {
    let [a, b, c] = triangle.points;
    let color = triangle.stroke_color.unwrap_or(triangle.color);
    let alpha = triangle.alpha;

    let mut path = Path2D::new();
    path.move_to(a)
        .line_to(b)
        .line_to(c)
        .stroke(triangle.stroke_width)
        .stroke_color(color)
        .alpha(alpha)
        .close();

    // TODO apply transform
    // if let Some(m) = matrix {
    //     path.transform(m);
    // }

    path.process(draw)
}
