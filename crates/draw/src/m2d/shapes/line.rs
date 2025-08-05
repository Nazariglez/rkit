use crate::{Draw2D, DrawPipelineId, DrawingInfo, Element2D, Transform2D};
use corelib::{
    gfx::Color,
    math::{Mat3, Vec2, bvec2, vec2},
};
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
        let p1 = self.p1;
        let p2 = self.p2;

        // calculate corners
        let dir = (p2 - p1).normalize_or_zero();
        let perp = vec2(-dir.y, dir.x) * (self.stroke_width * 0.5);
        let pa = p1 + perp;
        let pb = p1 - perp;
        let pc = p2 - perp;
        let pd = p2 + perp;

        let c = self.color.with_alpha(self.color.a * self.alpha);

        let indices = [0, 1, 2, 0, 2, 3];

        #[rustfmt::skip]
        let mut vertices = [
            pa.x, pa.y, c.r, c.g, c.b, c.a,
            pb.x, pb.y, c.r, c.g, c.b, c.a,
            pc.x, pc.y, c.r, c.g, c.b, c.a,
            pd.x, pd.y, c.r, c.g, c.b, c.a,
        ];

        let matrix = self
            .transform
            .map_or(Mat3::IDENTITY, |mut t| t.updated_mat3());

        draw.add_to_batch(DrawingInfo {
            pipeline: self.pip,
            vertices: &mut vertices,
            indices: &indices,
            transform: matrix,
            sprite: None,
        });
    }
}
