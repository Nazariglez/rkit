use crate::{Draw2D, DrawPipelineId, Element2D, Sprite, Transform2D};
use corelib::{
    gfx::Color,
    math::{IntoVec2, Mat3, Rect, Vec2, bvec2, vec2},
};
use macros::Drawable2D;

#[derive(Drawable2D)]
pub struct NineSlice2D {
    sprite: Sprite,
    position: Vec2,
    color: Color,
    alpha: f32,
    size: Option<Vec2>,

    #[pipeline_id]
    pip: DrawPipelineId,

    #[transform_2d]
    transform: Option<Transform2D>,
}

impl NineSlice2D {
    pub fn new(sprite: &Sprite) -> Self {
        Self {
            sprite: sprite.clone(),
            position: Vec2::ZERO,
            color: Color::WHITE,
            alpha: 1.0,
            size: None,
            pip: DrawPipelineId::Images,
            transform: None,
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

    pub fn size(&mut self, size: Vec2) -> &mut Self {
        self.size = Some(size);
        self
    }
}

impl Element2D for NineSlice2D {
    fn process(&self, draw: &mut Draw2D) {
        let Rect {
            size: sprite_size, ..
        } = self.sprite.frame();

        let sprite_origin = Vec2::ZERO;
        let size = self.size.unwrap_or(self.sprite.size());
        let (x, y) = (self.position.x, self.position.y);

        // Calculate slice dimensions (divide sprite into 3x3 grid)
        let slice_size = sprite_size / 3.0;

        // Calculate individual slice dimensions
        let left = slice_size.x;
        let right = slice_size.x;
        let top = slice_size.y;
        let bottom = slice_size.y;
        let center_w = size.x - (left + right);
        let center_h = size.y - (top + bottom);

        // Calculate center image dimensions
        let center_img_w = sprite_size.x - (left + right);
        let center_img_h = sprite_size.y - (top + bottom);

        let matrix = self
            .transform
            .map_or(Mat3::IDENTITY, |mut t| t.updated_mat3());

        draw.push_matrix(matrix);

        // top-left
        draw.image(&self.sprite)
            .color(self.color)
            .alpha(self.alpha)
            .translate(vec2(x, y))
            .size(vec2(left, top))
            .crop(sprite_origin, slice_size);

        // top-center
        draw.image(&self.sprite)
            .color(self.color)
            .alpha(self.alpha)
            .translate(vec2(x + left, y))
            .size(vec2(center_w, top))
            .crop(vec2(left, sprite_origin.y), vec2(center_img_w, top));

        // top-right
        draw.image(&self.sprite)
            .color(self.color)
            .alpha(self.alpha)
            .translate(vec2(x + left + center_w, y))
            .size(vec2(right, top))
            .crop(vec2(left + center_img_w, sprite_origin.y), vec2(right, top));

        // middle-left
        draw.image(&self.sprite)
            .color(self.color)
            .alpha(self.alpha)
            .translate(vec2(x, y + top))
            .size(vec2(left, center_h))
            .crop(vec2(sprite_origin.x, top), vec2(left, center_img_h));

        // center
        draw.image(&self.sprite)
            .color(self.color)
            .alpha(self.alpha)
            .translate(vec2(x + left, y + top))
            .size(vec2(center_w, center_h))
            .crop(vec2(left, top), vec2(center_img_w, center_img_h));

        // middle-right
        draw.image(&self.sprite)
            .color(self.color)
            .alpha(self.alpha)
            .translate(vec2(x + left + center_w, y + top))
            .size(vec2(right, center_h))
            .crop(vec2(left + center_img_w, top), vec2(right, center_img_h));

        // bottom-left
        draw.image(&self.sprite)
            .color(self.color)
            .alpha(self.alpha)
            .translate(vec2(x, y + top + center_h))
            .size(vec2(left, bottom))
            .crop(
                vec2(sprite_origin.x, top + center_img_h),
                vec2(left, bottom),
            );

        // bottom-center
        draw.image(&self.sprite)
            .color(self.color)
            .alpha(self.alpha)
            .translate(vec2(x + left, y + top + center_h))
            .size(vec2(center_w, bottom))
            .crop(vec2(left, top + center_img_h), vec2(center_img_w, bottom));

        // bottom-right
        draw.image(&self.sprite)
            .color(self.color)
            .alpha(self.alpha)
            .translate(vec2(x + left + center_w, y + top + center_h))
            .size(vec2(right, bottom))
            .crop(
                vec2(left + center_img_w, top + center_img_h),
                vec2(right, bottom),
            );

        draw.pop_matrix();
    }
}
