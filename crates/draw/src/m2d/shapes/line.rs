use crate::m2d::shapes::Path2D;
use crate::{Draw2D, DrawPipelineId, Element2D, Transform2D};
use core::gfx::Color;
use core::math::{bvec2, Vec2};
use macros::Drawable2D;

#[derive(Drawable2D)]
pub struct Line2D {
    p1: Vec2,
    p2: Vec2,
    color: Color,
    stroke_width: f32,
    alpha: f32,

    #[pipeline_id]
    pip: DrawPipelineId,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl Line2D {
    pub fn new(p1: Vec2, p2: Vec2) -> Self {
        Self {
            p1,
            p2,
            color: Color::WHITE,
            stroke_width: 1.0,
            alpha: 1.0,
            pip: DrawPipelineId::Shapes,
            transform: None,
        }
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }

    pub fn width(&mut self, width: f32) -> &mut Self {
        self.stroke_width = width;
        self
    }

    pub fn alpha(&mut self, alpha: f32) -> &mut Self {
        self.alpha = alpha;
        self
    }
}

impl Element2D for Line2D {
    fn process(&self, draw: &mut Draw2D) {
        let mut path = Path2D::new();
        path.transform = self.transform;
        path.pip = self.pip;

        path.move_to(self.p1)
            .line_to(self.p2)
            .stroke(self.stroke_width)
            .color(self.color.with_alpha(self.color.a * self.alpha))
            .close();

        path.process(draw)
    }
}
