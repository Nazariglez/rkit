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
}
