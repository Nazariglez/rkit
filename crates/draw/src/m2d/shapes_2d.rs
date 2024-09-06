use super::draw_2d::Element2D;
use crate::{Draw2D, DrawPipeline, DrawingInfo};
use core::gfx::Color;
use core::math::Vec2;

pub struct Triangle {
    points: [Vec2; 3],
    color: Color,
}

impl Triangle {
    pub fn new(p1: Vec2, p2: Vec2, p3: Vec2) -> Self {
        Self {
            points: [p1, p2, p3],
            color: Color::WHITE,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl Element2D for Triangle {
    fn process(&self, draw: &mut Draw2D) {
        let alpha = draw.alpha();

        // compute matrix

        let [a, b, c] = self.points;
        let color = self.color;

        #[rustfmt::skip]
        let vertices = [
            a.x, a.y, color.r, color.g, color.b, color.a * alpha,
            b.x, b.y, color.r, color.g, color.b, color.a * alpha,
            c.x, c.y, color.r, color.g, color.b, color.a * alpha,
        ];

        let indices = [0, 1, 2];

        draw.add_to_batch(DrawingInfo {
            pipeline: DrawPipeline::Shapes,
            vertices: &vertices,
            indices: &indices,
        })
    }
}
