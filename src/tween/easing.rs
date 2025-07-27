#![allow(unused)]
use std::f32::consts::PI;
use std::ops::{Add, Mul, Sub};
use std::process::Output;

pub type EaseFn = fn(f32) -> f32;

pub trait Interpolable: Copy + Sized {
    fn interpolate<F>(self, to: Self, progress: f32, easing: F) -> Self
    where
        F: Fn(f32) -> f32;
}

impl<T> Interpolable for T
where
    T: Copy
        + Mul<f32, Output = Self>
        + Mul<Self, Output = Self>
        + Add<Output = Self>
        + Sub<Output = Self>
        + Sized,
{
    #[inline(always)]
    fn interpolate<F>(self, to: Self, progress: f32, easing: F) -> Self
    where
        F: Fn(f32) -> f32,
    {
        self + ((to - self) * easing(progress))
    }
}

pub const LINEAR: EaseFn = |t: f32| -> f32 { t };

pub const IN_QUAD: EaseFn = |t: f32| -> f32 { t * t };

pub const OUT_QUAD: EaseFn = |t: f32| -> f32 { t * (2.0 - t) };

pub const IN_OUT_QUAD: EaseFn = |t: f32| -> f32 {
    let mut t = t * 2.0;
    if t < 1.0 {
        return 0.5 * t * t;
    }

    t -= 1.0;

    -0.5 * (t * (t - 2.0) - 1.0)
};

pub const IN_CUBIC: EaseFn = |t: f32| -> f32 { t * t * t };

pub const OUT_CUBIC: EaseFn = |t: f32| -> f32 { IN_CUBIC(t - 1.0) + 1.0 };

pub const IN_OUT_CUBIC: EaseFn = |t: f32| -> f32 {
    let mut t = t * 2.0;
    if t < 1.0 {
        return 0.5 * t * t * t;
    }

    t -= 2.0;
    0.5 * (t * t * t + 2.0)
};

pub const IN_QUART: EaseFn = |t: f32| -> f32 { t * t * t * t };

pub const OUT_QUART: EaseFn = |t: f32| -> f32 {
    let t = t - 1.0;
    1.0 - (t * t * t * t)
};

pub const IN_OUT_QUART: EaseFn = |t: f32| -> f32 {
    let mut t = t * 2.0;
    if t < 1.0 {
        return 0.5 * t * t * t * t;
    }
    t -= 2.0;
    -0.5 * (t * t * t * t - 2.0)
};

pub const IN_QUINT: EaseFn = |t: f32| -> f32 { t * t * t * t * t };

pub const OUT_QUINT: EaseFn = |t: f32| -> f32 {
    let t = t - 1.0;
    t * t * t * t * t + 1.0
};

pub const IN_OUT_QUINT: EaseFn = |t: f32| -> f32 {
    let mut t = t * 2.0;
    if t < 1.0 {
        return 0.5 * t * t * t * t * t;
    }
    t -= 2.0;
    0.5 * (t * t * t * t * t + 2.0)
};

pub const IN_SINE: EaseFn = |t: f32| -> f32 { 1.0 - ((t * PI) / 2.0).cos() };

pub const OUT_SINE: EaseFn = |t: f32| -> f32 { (t * PI / 2.0).sin() };

pub const IN_OUT_SINE: EaseFn = |t: f32| -> f32 { 0.5 * (1.0 - (PI * t).cos()) };

pub const IN_EXPO: EaseFn = |t: f32| -> f32 {
    if t == 0.0 {
        0.0
    } else {
        (1024.0f32).powf(t - 1.0)
    }
};

pub const OUT_EXPO: EaseFn = |t: f32| -> f32 {
    if t == 1.0 {
        1.0
    } else {
        1.0 - (2.0f32).powf(-10.0 * t)
    }
};

pub const IN_OUT_EXPO: EaseFn = |t: f32| -> f32 {
    if t == 0.0 {
        return 0.0;
    }

    if t == 1.0 {
        return 1.0;
    }

    let t = t * 2.0;
    if t < 1.0 {
        return 0.5 * (1024f32).powf(t - 1.0);
    }

    0.5 * (-(2.0f32).powf(-10.0 * (t - 1.0)) + 2.0)
};

pub const IN_CIRC: EaseFn = |t: f32| -> f32 { 1.0 - (1.0 - t * t).sqrt() };

pub const OUT_CIRC: EaseFn = |t: f32| -> f32 {
    let t = t - 1.0;
    (1.0 - (t * t)).sqrt()
};

pub const IN_OUT_CIRC: EaseFn = |t: f32| -> f32 {
    let t = t * 2.0;
    if t < 1.0 {
        return -0.5 * ((1.0 - t * t).sqrt() - 1.0);
    }
    0.5 * ((1.0 - (t - 2.0) * (t - 2.0)).sqrt() + 1.0)
};

pub const IN_ELASTIC: EaseFn = |t: f32| -> f32 {
    if t == 0.0 || t == 1.0 {
        return t;
    }

    let a = 1.0;
    let p = 0.4;
    let s = p / 4.0;

    -(a * (2.0f32).powf(10.0 * (t - 1.0)) * (((t - 1.0) - s) * (2.0 * PI) / p).sin())
};

pub const OUT_ELASTIC: EaseFn = |t: f32| -> f32 {
    if t == 0.0 || t == 1.0 {
        return t;
    }

    let a = 1.0;
    let p = 0.4;
    let s = p / 4.0;

    (a * (2.0f32).powf(-10.0 * t) * ((t - s) * (2.0 * PI) / p).sin() + 1.0)
};

pub const IN_OUT_ELASTIC: EaseFn = |t: f32| -> f32 {
    if t == 0.0 || t == 1.0 {
        return t;
    }

    let a = 1.0;
    let p = 0.4;
    let s = p * (1.0f32 / a).asin() / (2.0 * PI);

    let t = t * 2.0;
    if t < 1.0 {
        -0.5 * (a * (2.0f32).powf(10.0 * (t - 1.0)) * (((t - 1.0) - s) * (2.0 * PI) / p).sin())
    } else {
        a * (2.0f32).powf(-10.0 * (t - 1.0)) * (((t - 1.0) - s) * (2.0 * PI) / p).sin() * 0.5 + 1.0
    }
};
pub const IN_BACK: EaseFn = |t: f32| -> f32 {
    let m = 1.70158;
    t * t * ((m + 1.0) * t - m)
};

pub const OUT_BACK: EaseFn = |t: f32| -> f32 {
    let t = t - 1.0;
    let m = 1.70158;
    t * t * ((m + 1.0) * t + m) + 1.0
};

pub const IN_OUT_BACK: EaseFn = |t: f32| -> f32 {
    let m = 1.70158;
    let s = m * 1.525;
    let t = t * 2.0;
    if t < 1.0 {
        0.5 * (t * t * ((s + 1.0) * t - s))
    } else {
        0.5 * ((t - 2.0) * (t - 2.0) * ((s + 1.0) * (t - 2.0) + s) + 2.0)
    }
};

pub const IN_BOUNCE: EaseFn = |t: f32| -> f32 { 1.0 - OUT_BOUNCE(1.0 - t) };

pub const OUT_BOUNCE: EaseFn = |t: f32| -> f32 {
    let m = 2.75;
    let m1 = 7.5625;
    if t < (1.0 / m) {
        m1 * t * t
    } else if t < (2.0 / m) {
        let t = (t - (1.5 / m));
        m1 * t * t + 0.75
    } else if t < (2.5 / m) {
        let t = (t - (2.25 / m));
        m1 * t * t + 0.9375
    } else {
        let t = t - (2.625 / m);
        m1 * t * t + 0.984375
    }
};

pub const IN_OUT_BOUNCE: EaseFn = |t: f32| -> f32 {
    if t < 0.5 {
        IN_BOUNCE(t * 2.0) * 0.5
    } else {
        OUT_BOUNCE(t * 2.0 - 1.0) * 0.5 + 0.5
    }
};

#[cfg(test)]
mod test {
    use super::*;
    use crate::math::Vec2;

    #[test]
    fn test_linear_interpolate_0() {
        let from = 0.0;
        let to = 100.0;
        let total_time = 10.0;
        let elapsed_time = 0.0;
        let value = from.interpolate(to, elapsed_time / total_time, LINEAR);
        assert_eq!(value, 0.0)
    }

    #[test]
    fn test_linear_interpolate_05() {
        let from = 0.0;
        let to = 100.0;
        let total_time = 10.0;
        let elapsed_time = 5.0;
        let value = from.interpolate(to, elapsed_time / total_time, LINEAR);
        assert_eq!(value, 50.0)
    }

    #[test]
    fn test_linear_interpolate_1() {
        let from = 0.0;
        let to = 100.0;
        let total_time = 10.0;
        let elapsed_time = 10.0;
        let value = from.interpolate(to, elapsed_time / total_time, LINEAR);
        assert_eq!(value, 100.0)
    }

    #[test]
    fn test_vec2_interpolate_0() {
        let from = Vec2::ZERO;
        let to = Vec2::splat(100.0);
        let total_time = 10.0;
        let elapsed_time = 0.0;
        let value = from.interpolate(to, elapsed_time / total_time, LINEAR);
        assert_eq!(value, Vec2::ZERO)
    }

    #[test]
    fn test_vec2_interpolate_05() {
        let from = Vec2::ZERO;
        let to = Vec2::splat(100.0);
        let total_time = 10.0;
        let elapsed_time = 5.0;
        let value = from.interpolate(to, elapsed_time / total_time, LINEAR);
        assert_eq!(value, Vec2::splat(50.0))
    }

    #[test]
    fn test_vec2_interpolate_1() {
        let from = Vec2::ZERO;
        let to = Vec2::splat(100.0);
        let total_time = 10.0;
        let elapsed_time = 10.0;
        let value = from.interpolate(to, elapsed_time / total_time, LINEAR);
        assert_eq!(value, Vec2::splat(100.0))
    }
}
