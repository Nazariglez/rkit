use super::draw_2d::Element2D;
use crate::{Draw2D, DrawPipeline, DrawingInfo};
use core::gfx::{Color, RenderPipeline};
use core::math::{Mat3, Vec2};

pub fn create_pixel_pipeline() -> Result<RenderPipeline, String> {
    todo!()
}

pub struct Pixel {
    pos: Vec2,
    color: Color,
}

impl Pixel {
    pub fn new(pos: Vec2) -> Self {
        Self {
            pos,
            color: Color::WHITE,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl Element2D for Pixel {
    fn process(&self, draw: &mut Draw2D) {
        // compute matrix
        draw.add_to_batch(DrawingInfo {
            pipeline: DrawPipeline::Pixel,
            vertices: &[self.pos.x, self.pos.y],
            indices: &[0],
            offset: 2,
            transform: Mat3::IDENTITY,
        })
    }
}
