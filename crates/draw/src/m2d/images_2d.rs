use crate::{Draw2D, Element2D};
use core::gfx::{Color, Texture};
use core::math::Vec2;

pub struct Image {
    texture: Texture,
    position: Vec2,
    color: Color,
    alpha: f32,
}

impl Image {
    pub fn new(texture: &Texture) -> Self {
        Self {
            texture: texture.clone(),
            position: Vec2::ZERO,
            color: Color::WHITE,
            alpha: 1.0,
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
        // TODO (on add to batch creatr bind_group if necessary)
    }
}
