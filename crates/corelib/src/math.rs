pub use glam::*;

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub origin: Vec2,
    pub size: Vec2,
}

impl Rect {
    #[inline]
    pub const fn new(origin: Vec2, size: Vec2) -> Self {
        Self { origin, size }
    }

    #[inline]
    pub const fn size(&self) -> Vec2 {
        self.size
    }

    #[inline]
    pub fn from_center(center: Vec2, size: Vec2) -> Self {
        let origin = center - size * 0.5;
        Self { origin, size }
    }

    #[inline]
    pub fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self {
            origin: min,
            size: max - min,
        }
    }

    #[inline]
    pub fn min(&self) -> Vec2 {
        self.origin
    }

    #[inline]
    pub fn max(&self) -> Vec2 {
        self.origin + self.size
    }

    #[inline]
    pub fn center(&self) -> Vec2 {
        self.origin + self.size * 0.5
    }

    #[inline]
    pub fn contains(&self, point: Vec2) -> bool {
        let min = self.min();
        let max = self.max();
        point.x >= min.x && point.x <= max.x && point.y >= min.y && point.y <= max.y
    }

    #[inline]
    pub fn intersects(&self, other: &Rect) -> bool {
        let self_min = self.min();
        let self_max = self.max();
        let other_min = other.min();
        let other_max = other.max();

        !(self_max.x < other_min.x
            || self_min.x > other_max.x
            || self_max.y < other_min.y
            || self_min.y > other_max.y)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size.x <= 0.0 || self.size.y <= 0.0
    }

    #[inline]
    pub fn width(&self) -> f32 {
        self.size.x
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.size.y
    }

    #[inline]
    pub fn x(&self) -> f32 {
        self.origin.x
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.origin.y
    }
}

pub trait IntoVec2 {
    fn into_vec2(self) -> Vec2;
}

impl IntoVec2 for Vec2 {
    #[inline(always)]
    fn into_vec2(self) -> Vec2 {
        self
    }
}
impl IntoVec2 for (f32, f32) {
    #[inline(always)]
    fn into_vec2(self) -> Vec2 {
        self.into()
    }
}
impl IntoVec2 for [f32; 2] {
    #[inline(always)]
    fn into_vec2(self) -> Vec2 {
        self.into()
    }
}
impl IntoVec2 for f32 {
    #[inline(always)]
    fn into_vec2(self) -> Vec2 {
        Vec2::splat(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_point() {
        let rect = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));

        // Point inside the rectangle
        assert!(rect.contains(Vec2::new(5.0, 5.0)));

        // Point on the edge of the rectangle
        assert!(rect.contains(Vec2::new(0.0, 0.0)));
        assert!(rect.contains(Vec2::new(10.0, 10.0)));

        // Point outside the rectangle
        assert!(!rect.contains(Vec2::new(-1.0, 5.0)));
        assert!(!rect.contains(Vec2::new(11.0, 5.0)));
        assert!(!rect.contains(Vec2::new(5.0, -1.0)));
        assert!(!rect.contains(Vec2::new(5.0, 11.0)));
    }

    #[test]
    fn test_intersects_rect() {
        let rect1 = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let rect2 = Rect::new(Vec2::new(5.0, 5.0), Vec2::new(10.0, 10.0));
        let rect3 = Rect::new(Vec2::new(11.0, 11.0), Vec2::new(10.0, 10.0));
        let rect4 = Rect::new(Vec2::new(10.0, 0.0), Vec2::new(5.0, 5.0));

        assert!(rect1.intersects(&rect2));
        assert!(!rect1.intersects(&rect3));
        assert!(rect1.intersects(&rect4));
    }

    #[test]
    fn test_into_vec2_from_vec2() {
        let v = Vec2::new(1.0, 2.0);
        assert_eq!(v.into_vec2(), Vec2::new(1.0, 2.0));
    }

    #[test]
    fn test_into_vec2_from_tuple() {
        let v = (3.0, 4.0).into_vec2();
        assert_eq!(v, Vec2::new(3.0, 4.0));
    }

    #[test]
    fn test_into_vec2_from_array() {
        let v = [5.0, 6.0].into_vec2();
        assert_eq!(v, Vec2::new(5.0, 6.0));
    }

    #[test]
    fn test_into_vec2_from_f32() {
        let v = 7.0_f32.into_vec2();
        assert_eq!(v, Vec2::new(7.0, 7.0));
    }
}
