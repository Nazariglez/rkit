use corelib::math::{vec2, BVec2, Mat3, Vec2};
use num::Zero;
use smallvec::SmallVec;

#[derive(Clone)]
pub(crate) struct Mat3Stack {
    base: Mat3,
    stack: SmallVec<Mat3, 30>,
}

impl Default for Mat3Stack {
    fn default() -> Self {
        Self {
            base: Mat3::IDENTITY,
            stack: SmallVec::default(),
        }
    }
}

impl Mat3Stack {
    pub fn matrix(&self) -> Mat3 {
        *self.stack.last().unwrap_or(&self.base)
    }

    pub fn set_matrix(&mut self, m: Mat3) {
        let mat = self.stack.last_mut().unwrap_or(&mut self.base);
        *mat = m;
    }

    pub fn push(&mut self, m: Mat3) {
        self.stack.push(self.matrix() * m);
    }

    pub fn pop(&mut self) {
        self.stack.pop();
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Transform2D {
    translation: Vec2,
    size: Vec2,
    scale: Vec2,
    anchor: Vec2,
    pivot: Vec2,
    rotation: f32,
    flip: BVec2,

    skew: Vec2,
    skew_cache_col_0: Option<Vec2>,
    skew_cache_col_1: Option<Vec2>,

    dirty: bool,
    pub(crate) mat3: Mat3,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::new()
    }
}

impl Transform2D {
    pub fn builder() -> Transform2DBuilder {
        Transform2DBuilder::default()
    }

    pub fn new() -> Self {
        Self {
            translation: Default::default(),
            size: Default::default(),
            scale: Vec2::splat(1.0),
            anchor: Default::default(),
            pivot: Default::default(),
            rotation: 0.0,
            flip: Default::default(),
            skew: Default::default(),
            skew_cache_col_0: None,
            skew_cache_col_1: None,
            dirty: true,
            mat3: Mat3::IDENTITY,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn update(&mut self) {
        if self.dirty {
            self.mat3 = update_transform(self);
            self.dirty = false
        }
    }

    pub fn updated_mat3(&mut self) -> Mat3 {
        self.update();
        self.mat3
    }

    pub fn as_mat3(&self) -> Mat3 {
        debug_assert!(!self.is_dirty(), "Transformation is dirty.");
        self.mat3
    }

    pub fn position(&self) -> Vec2 {
        self.translation
    }
    pub fn rotation(&self) -> f32 {
        self.rotation
    }
    pub fn skew(&self) -> Vec2 {
        self.skew
    }
    pub fn anchor(&self) -> Vec2 {
        self.anchor
    }
    pub fn pivot(&self) -> Vec2 {
        self.pivot
    }
    pub fn size(&self) -> Vec2 {
        self.size
    }
    pub fn scale(&self) -> Vec2 {
        self.scale
    }
    pub fn flip(&self) -> BVec2 {
        self.flip
    }

    pub fn set_translation(&mut self, position: Vec2) -> &mut Self {
        self.translation = position;
        self.dirty = true;
        self
    }
    pub fn set_rotation(&mut self, rotation: f32) -> &mut Self {
        self.rotation = rotation;
        self.skew_cache_col_0 = None;
        self.skew_cache_col_1 = None;
        self.dirty = true;
        self
    }
    pub fn set_skew(&mut self, skew: Vec2) -> &mut Self {
        self.skew = skew;
        self.skew_cache_col_0 = None;
        self.skew_cache_col_1 = None;
        self.dirty = true;
        self
    }
    pub fn set_anchor(&mut self, anchor: Vec2) -> &mut Self {
        self.anchor = anchor;
        self.dirty = true;
        self
    }
    pub fn set_pivot(&mut self, pivot: Vec2) -> &mut Self {
        self.pivot = pivot;
        self.dirty = true;
        self
    }
    pub fn set_origin(&mut self, origin: Vec2) -> &mut Self {
        self.set_anchor(origin).set_pivot(origin)
    }
    pub fn set_flip(&mut self, flip: BVec2) -> &mut Self {
        self.flip = flip;
        self.dirty = true;
        self
    }
    pub fn set_size(&mut self, size: Vec2) -> &mut Self {
        self.size = size;
        self.dirty = true;
        self
    }
    pub fn set_scale(&mut self, scale: Vec2) -> &mut Self {
        self.scale = scale;
        self.dirty = true;
        self
    }
}

#[derive(Default)]
pub struct Transform2DBuilder {
    transform: Transform2D,
}

impl Transform2DBuilder {
    pub fn set_translation(mut self, position: Vec2) -> Self {
        self.transform.set_translation(position);
        self
    }

    pub fn set_rotation(mut self, rotation: f32) -> Self {
        self.transform.set_rotation(rotation);
        self
    }

    pub fn set_scale(mut self, scale: Vec2) -> Self {
        self.transform.set_scale(scale);
        self
    }

    pub fn set_size(mut self, size: Vec2) -> Self {
        self.transform.set_size(size);
        self
    }

    pub fn set_anchor(mut self, anchor: Vec2) -> Self {
        self.transform.set_anchor(anchor);
        self
    }

    pub fn set_pivot(mut self, pivot: Vec2) -> Self {
        self.transform.set_pivot(pivot);
        self
    }

    pub fn set_origin(self, origin: Vec2) -> Self {
        self.set_pivot(origin).set_anchor(origin)
    }

    pub fn set_flip(mut self, flip: BVec2) -> Self {
        self.transform.set_flip(flip);
        self
    }

    pub fn set_skew(mut self, skew: Vec2) -> Self {
        self.transform.set_skew(skew);
        self
    }

    pub fn build(mut self) -> Transform2D {
        self.transform.update();
        self.transform
    }
}

impl From<Transform2DBuilder> for Transform2D {
    fn from(builder: Transform2DBuilder) -> Self {
        builder.build()
    }
}

fn update_transform(t: &mut Transform2D) -> Mat3 {
    let skew_col_0 = t.skew_cache_col_0.unwrap_or_else(|| {
        vec2(
            (t.rotation + t.skew.x).cos(),
            -(t.rotation - t.skew.y).sin(),
        )
    });
    let skew_col_1 = t
        .skew_cache_col_1
        .unwrap_or_else(|| vec2((t.rotation + t.skew.x).sin(), (t.rotation - t.skew.y).cos()));

    let scale = t.scale * flip_to_mul(t.flip);
    let anchor = minus_flip(t.flip, t.anchor);
    let pivot = minus_flip(t.flip, t.pivot);

    let col_0 = skew_col_0 * scale;
    let col_1 = skew_col_1 * scale;

    let anchor = anchor * t.size;
    let pivot = pivot * t.size;

    let mut row_2 = t.translation - anchor * scale + pivot * scale;
    if !pivot.x.is_zero() || !pivot.y.is_zero() {
        row_2 -= vec2(pivot.dot(col_0), pivot.dot(col_1));
    }

    t.skew_cache_col_0 = Some(skew_col_0);
    t.skew_cache_col_1 = Some(skew_col_1);
    t.dirty = false;

    let Vec2 { x: m00, y: m10 } = col_0;
    let Vec2 { x: m01, y: m11 } = col_1;
    let Vec2 { x: m20, y: m21 } = row_2;

    Mat3::from_cols_array(&[m00, m01, 0.0, m10, m11, 0.0, m20, m21, 1.0])
}

fn flip_to_mul(b: BVec2) -> Vec2 {
    Vec2::select(b, Vec2::splat(-1.0), Vec2::splat(1.0))
}

fn minus_flip(b: BVec2, v: Vec2) -> Vec2 {
    Vec2::select(b, Vec2::splat(1.0) - v, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mat3_stack_default() {
        let stack = Mat3Stack::default();
        assert_eq!(stack.matrix(), Mat3::IDENTITY);
    }

    #[test]
    fn test_mat3_stack_push_and_pop() {
        let mut stack = Mat3Stack::default();
        let mat = Mat3::from_scale(vec2(2.0, 2.0));

        stack.push(mat);
        assert_eq!(stack.matrix(), mat);

        stack.pop();
        assert_eq!(stack.matrix(), Mat3::IDENTITY);
    }

    #[test]
    fn test_mat3_stack_clear() {
        let mut stack = Mat3Stack::default();
        let mat = Mat3::from_scale(vec2(2.0, 2.0));

        stack.push(mat);
        stack.clear();
        assert_eq!(stack.matrix(), Mat3::IDENTITY);
        assert!(stack.stack.is_empty());
    }

    #[test]
    fn test_mat3_stack_set_matrix() {
        let mut stack = Mat3Stack::default();
        let mat = Mat3::from_scale(vec2(3.0, 3.0));

        stack.set_matrix(mat);
        assert_eq!(stack.matrix(), mat);
    }

    #[test]
    fn test_transform2d_default() {
        let transform = Transform2D::default();
        assert_eq!(transform.translation, Vec2::ZERO);
        assert_eq!(transform.size, Vec2::ZERO);
        assert_eq!(transform.scale, Vec2::splat(1.0));
        assert_eq!(transform.rotation, 0.0);
        assert_eq!(transform.anchor, Vec2::ZERO);
        assert_eq!(transform.pivot, Vec2::ZERO);
        assert_eq!(transform.flip, BVec2::new(false, false));
    }

    #[test]
    fn test_transform2d_set_position() {
        let mut transform = Transform2D::default();
        transform.set_translation(vec2(10.0, 20.0));
        assert_eq!(transform.translation, vec2(10.0, 20.0));
    }

    #[test]
    fn test_transform2d_set_rotation() {
        let mut transform = Transform2D::default();
        transform.set_rotation(1.57);
        assert_eq!(transform.rotation, 1.57);
        assert!(transform.skew_cache_col_0.is_none());
        assert!(transform.skew_cache_col_1.is_none());
    }

    #[test]
    fn test_transform2d_set_scale() {
        let mut transform = Transform2D::default();
        transform.set_scale(vec2(2.0, 3.0));
        assert_eq!(transform.scale, vec2(2.0, 3.0));
    }

    #[test]
    fn test_transform2d_as_mat3() {
        let mut transform = Transform2D::default();
        transform.set_translation(vec2(10.0, 20.0));
        transform.set_rotation(0.0);
        transform.set_scale(vec2(1.0, 1.0));

        let mat = transform.updated_mat3();
        assert_eq!(mat, Mat3::from_translation(vec2(10.0, 20.0)));
    }

    #[test]
    fn test_transform2d_flip() {
        let mut transform = Transform2D::default();
        transform.set_flip(BVec2::new(true, false));
        assert_eq!(transform.flip, BVec2::new(true, false));
    }
}
