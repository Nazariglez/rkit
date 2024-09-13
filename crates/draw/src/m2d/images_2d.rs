use crate::{Draw2D, DrawPipeline, DrawingInfo, Element2D, Sprite};
use core::gfx::Color;
use core::math::{Mat3, UVec2, Vec2};

// pos(f32x2) + uvs(f32x2) + color(f32x4)
const VERTICES_OFFSET: usize = 8;

pub struct Image {
    sprite: Sprite,
    position: Vec2,
    color: Color,
    alpha: f32,
    transform: Mat3,
}

impl Image {
    pub fn new(sprite: &Sprite) -> Self {
        Self {
            sprite: sprite.clone(),
            position: Vec2::ZERO,
            color: Color::WHITE,
            alpha: 1.0,
            transform: Mat3::IDENTITY,
        }
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }

    pub fn alpha(&mut self, alpha: f32) -> &mut Self {
        self.alpha = alpha;
        self
    }

    pub fn position(&mut self, pos: Vec2) -> &mut Self {
        self.position = pos;
        self
    }
}

impl Element2D for Image {
    fn process(&self, draw: &mut Draw2D) {
        // TODO (on add to batch creatr bind_group if necessary)
        let c = self.color.with_alpha(self.color.a * self.alpha);
        let Vec2 { x: x1, y: y1 } = self.position;
        let UVec2 { x: x2, y: y2 } = self.sprite.size();
        let (x2, y2) = (x2 as f32, y2 as f32);

        let (u1, v1, u2, v2) = (0.0, 0.0, 1.0, 1.0);

        #[rustfmt::skip]
        let vertices = [
            x1, y1, u1, v1, c.r, c.g, c.b, c.a,
            x2, y1, u2, v1, c.r, c.g, c.b, c.a,
            x1, y2, u1, v2, c.r, c.g, c.b, c.a,
            x2, y2, u2, v2, c.r, c.g, c.b, c.a,
        ];

        let indices = [0, 1, 2, 2, 1, 3];

        draw.add_to_batch(DrawingInfo {
            pipeline: DrawPipeline::Images,
            vertices: &vertices,
            indices: &indices,
            offset: VERTICES_OFFSET,
            transform: self.transform,
            sprite: Some(&self.sprite),
        })
    }
}
