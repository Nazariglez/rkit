use std::ops::{Add, Mul};

use crate::gfx::{self, Color};
use crate::math::Vec2;
use crate::random;
use crate::tween::{EaseFn, Interpolable, LINEAR};

#[derive(Debug, Default, Clone)]
pub enum Curve {
    #[default]
    Linear,
    Custom(Vec<(f32, f32)>),
}

impl Curve {
    pub fn apply<T>(&self, start: T, end: T, progress: f32) -> T
    where
        T: Interpolable + Mul<f32, Output = T> + Add<Output = T>,
    {
        match self {
            Curve::Linear => start.interpolate(end, progress, LINEAR),
            Curve::Custom(keyframes) => {
                start.interpolate(end, progress, |t: f32| linear_keyframes(t, keyframes))
            }
        }
    }
}

fn linear_keyframes(t: f32, keyframes: &[(f32, f32)]) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if keyframes.is_empty() {
        return t;
    }

    debug_assert!(
        keyframes.windows(2).all(|w| w[0].0 <= w[1].0),
        "keyframes must be sorted by time"
    );

    // first point time is by default (0,0)
    let mut time = 0.0;
    let mut value = 0.0;

    // inteprolate between keyframes
    for &(kf_time, kf_value) in keyframes {
        // interpolate if t is <= than the keyframe
        if t <= kf_time {
            // if it's the same ketyframe time then just return value
            let diff_time = kf_time - time;
            if diff_time <= 0.0 {
                return kf_value;
            }

            // interpolate
            let local_t = (t - time) / diff_time;
            return value + (kf_value - value) * local_t;
        }

        // otherwise move forward
        time = kf_time;
        value = kf_value;
    }

    // if t is past the last keyframe then we interpolate to (1,1)
    if time < 1.0 {
        let remaining_time = 1.0 - time;
        let local_t = (t - time) / remaining_time;
        return value + (1.0 - value) * local_t;
    }

    value
}

pub enum Value<T: Interpolable> {
    Fixed(T),
    Range { min: T, max: T },
}

impl<T> Value<T>
where
    T: Interpolable,
{
    pub fn val(&self) -> T {
        match self {
            Value::Fixed(t) => *t,
            Value::Range { min, max } => min.interpolate(*max, random::r#gen(), LINEAR),
        }
    }

    pub fn min(&self) -> T {
        match self {
            Value::Fixed(t) => *t,
            Value::Range { min, .. } => *min,
        }
    }

    pub fn max(&self) -> T {
        match self {
            Value::Fixed(t) => *t,
            Value::Range { max, .. } => *max,
        }
    }
}

pub struct Attr<T: Interpolable> {
    pub initial: Value<T>,
    pub behavior: Option<Behavior<T>>,
}

impl<T> Attr<T>
where
    T: Interpolable + Mul<f32, Output = T> + Add<Output = T>,
{
    pub fn init(&self) -> T {
        self.initial.val()
    }

    pub fn apply(&self, from: T, delta: f32, progress: f32) -> T {
        match &self.behavior {
            Some(Behavior::Fixed { start, end, curve }) => curve.apply(*start, *end, progress),
            Some(Behavior::Increment(val)) => from + *val * delta,
            None => from,
        }
    }
}

pub enum Behavior<T: Interpolable> {
    Fixed { start: T, end: T, curve: Curve },
    Increment(T),
}

pub struct Effect {
    pub pos: Vec2,
    pub emitters: Vec<Emitter>,
}

pub enum EmitterKind {
    Square(Vec2),
}

pub enum SortBy {
    SpawnBottom,
    SpawnTop,
    AxisYAsc,
    AxisYDesc,
}

pub struct Gravity {
    pub angle: f32,
    pub amount: f32,
}

pub struct Emitter {
    pub id: String,
    pub kind: EmitterKind,
    pub offset: Vec2,
    pub index: f32,
    pub particles_per_wave: usize,
    pub wave_time: f32,
    pub delay: f32,
    pub repeat: Option<usize>,
    pub gravity: Gravity,
    pub sort: Option<SortBy>,
    // pub blend_mode: todo!(),
    pub attributes: Attributes,
}

pub struct Attributes {
    pub textures: Vec<gfx::Texture>,
    pub lifetime: Value<f32>,
    pub scale_x: Attr<f32>,
    pub scale_y: Attr<f32>,
    pub red: Attr<f32>,
    pub blue: Attr<f32>,
    pub green: Attr<f32>,
    pub alpha: Attr<f32>,
    pub speed: Attr<f32>,
    pub rotation: Attr<f32>,
    pub angle: Attr<f32>,
}

// https://github.com/Nazariglez/perenquen/blob/master/src/particle/ParticleEmitter.js
// https://github.com/pixijs-userland/particle-emitter?tab=readme-ov-file
