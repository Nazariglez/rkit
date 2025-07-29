use std::{
    array,
    f32::consts::TAU,
    ops::{Add, Mul},
};

use corelib::math::{Vec3, vec3};
use draw::Draw2D;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumIter};

use crate::{
    ecs::prelude::*,
    gfx::Color,
    math::{Vec2, vec2},
    random,
    tween::*,
};

// TODO: add events for when it ends, repeats, etc...

#[derive(SystemSet, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ParticlesSysSet;

#[derive(Debug, Clone, Copy)]
pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn apply(&self, app: &mut App) {
        app.add_resource(Particles::default())
            .add_systems(OnUpdate, update_system.in_set(ParticlesSysSet))
            .configure_sets(OnUpdate, ParticlesSysSet);
    }
}

#[derive(Resource, Default, Clone, Deref)]
pub struct Particles(FxHashMap<String, ParticleFxConfig>);

impl Particles {
    pub fn create_component(&self, id: &str, pos: Vec2) -> Option<ParticleFx> {
        self.0.get(id).map(|config| ParticleFx {
            id: id.to_string(),
            pos,
            emitters: vec![ParticleEmitter::default(); config.emitters.len()],
            spawning: false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, AsRefStr, EnumIter, Serialize, Deserialize)]
pub enum Curve {
    Linear,
    InQuad,
    OutQuad,
    InOutQuad,
    InCubic,
    OutCubic,
    InOutCubic,
    InQuart,
    OutQuart,
    InOutQuart,
    InQuint,
    OutQuint,
    InOutQuint,
    InSine,
    OutSine,
    InOutSine,
    InExpo,
    OutExpo,
    InOutExpo,
    InCirc,
    OutCirc,
    InOutCirc,
    InElastic,
    OutElastic,
    InOutElastic,
    InBack,
    OutBack,
    InOutBack,
    InBounce,
    OutBounce,
    InOutBounce,
    Custom(Vec<(f32, f32)>),
}

impl Curve {
    #[inline(always)]
    pub fn apply<T>(&self, start: T, end: T, progress: f32) -> T
    where
        T: Interpolable + Mul<f32, Output = T> + Add<Output = T>,
    {
        match self {
            Curve::Linear => start.interpolate(end, progress, LINEAR),
            Curve::InQuad => start.interpolate(end, progress, IN_QUAD),
            Curve::OutQuad => start.interpolate(end, progress, OUT_QUAD),
            Curve::InOutQuad => start.interpolate(end, progress, IN_OUT_QUAD),
            Curve::InCubic => start.interpolate(end, progress, IN_CUBIC),
            Curve::OutCubic => start.interpolate(end, progress, OUT_CUBIC),
            Curve::InOutCubic => start.interpolate(end, progress, IN_OUT_CUBIC),
            Curve::InQuart => start.interpolate(end, progress, IN_QUART),
            Curve::OutQuart => start.interpolate(end, progress, OUT_QUART),
            Curve::InOutQuart => start.interpolate(end, progress, IN_OUT_QUART),
            Curve::InQuint => start.interpolate(end, progress, IN_QUINT),
            Curve::OutQuint => start.interpolate(end, progress, OUT_QUINT),
            Curve::InOutQuint => start.interpolate(end, progress, IN_OUT_QUINT),
            Curve::InSine => start.interpolate(end, progress, IN_SINE),
            Curve::OutSine => start.interpolate(end, progress, OUT_SINE),
            Curve::InOutSine => start.interpolate(end, progress, IN_OUT_SINE),
            Curve::InExpo => start.interpolate(end, progress, IN_EXPO),
            Curve::OutExpo => start.interpolate(end, progress, OUT_EXPO),
            Curve::InOutExpo => start.interpolate(end, progress, IN_OUT_EXPO),
            Curve::InCirc => start.interpolate(end, progress, IN_CIRC),
            Curve::OutCirc => start.interpolate(end, progress, OUT_CIRC),
            Curve::InOutCirc => start.interpolate(end, progress, IN_OUT_CIRC),
            Curve::InElastic => start.interpolate(end, progress, IN_ELASTIC),
            Curve::OutElastic => start.interpolate(end, progress, OUT_ELASTIC),
            Curve::InOutElastic => start.interpolate(end, progress, IN_OUT_ELASTIC),
            Curve::InBack => start.interpolate(end, progress, IN_BACK),
            Curve::OutBack => start.interpolate(end, progress, OUT_BACK),
            Curve::InOutBack => start.interpolate(end, progress, IN_OUT_BACK),
            Curve::InBounce => start.interpolate(end, progress, IN_BOUNCE),
            Curve::OutBounce => start.interpolate(end, progress, OUT_BOUNCE),
            Curve::InOutBounce => start.interpolate(end, progress, IN_OUT_BOUNCE),
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

#[derive(Debug, Clone, Copy, PartialEq, AsRefStr, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attr<T: Interpolable> {
    pub initial: Value<T>,
    pub behavior: Option<Behavior<T>>,
}

impl<T> Attr<T>
where
    T: Interpolable + Mul<f32, Output = T> + Add<Output = T>,
{
    #[inline(always)]
    pub fn init(&self) -> T {
        self.initial.val()
    }

    #[inline(always)]
    pub fn apply(&self, initial: T, current: T, delta: f32, progress: f32) -> T {
        match &self.behavior {
            Some(Behavior::To { value, curve }) => curve.apply(initial, *value, progress),
            Some(Behavior::Increment(val)) => current + *val * delta,
            None => current,
        }
    }
}

#[derive(Debug, Clone, PartialEq, AsRefStr, Serialize, Deserialize)]
pub enum Behavior<T: Interpolable> {
    To { value: T, curve: Curve },
    Increment(T),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorAttr {
    pub initial: Value<Vec3>,
    pub behavior: Option<ColorBehavior>,
}

impl ColorAttr {
    #[inline(always)]
    pub fn init(&self) -> Vec3 {
        match self.initial {
            Value::Fixed(t) => t,
            Value::Range { min, max } => {
                let r = min.x.interpolate(max.x, random::r#gen(), LINEAR);
                let g = min.y.interpolate(max.y, random::r#gen(), LINEAR);
                let b = min.z.interpolate(max.z, random::r#gen(), LINEAR);
                vec3(r, g, b)
            }
        }
    }

    #[inline(always)]
    pub fn apply(&self, initial: Vec3, current: Vec3, delta: f32, progress: f32) -> Vec3 {
        match &self.behavior {
            Some(ColorBehavior::Simple(behavior)) => match behavior {
                Behavior::To { value, curve } => curve.apply(initial, *value, progress),
                Behavior::Increment(val) => current + *val * delta,
            },
            Some(ColorBehavior::ByChannel { red, green, blue }) => {
                let base = [red, green, blue];
                let rgb = array::from_fn(|i| match base[i] {
                    Some(Behavior::To { value, curve }) => {
                        curve.apply(initial[i], *value, progress)
                    }
                    Some(Behavior::Increment(val)) => current[i] + *val * delta,
                    None => current[i],
                });
                rgb.into()
            }
            None => current,
        }
    }
}

#[derive(Debug, Clone, PartialEq, AsRefStr, Serialize, Deserialize)]
pub enum ColorBehavior {
    Simple(Behavior<Vec3>),
    ByChannel {
        red: Option<Behavior<f32>>,
        green: Option<Behavior<f32>>,
        blue: Option<Behavior<f32>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleFxConfig {
    pub emitters: Vec<EmitterConfig>,
}

impl Default for ParticleFxConfig {
    fn default() -> Self {
        Self {
            emitters: vec![EmitterConfig::default()],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, AsRefStr, Serialize, Deserialize)]
pub enum EmitterKind {
    Point,
    Rect(Vec2),
    Circle(f32),
    Ring { radius: f32, width: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SortBy {
    SpawnBottom,
    SpawnTop,
    AxisYAsc,
    AxisYDesc,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Gravity {
    pub angle: f32,
    pub amount: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitterConfig {
    pub id: String,
    pub kind: EmitterKind,
    pub offset: Vec2,
    pub index: f32,
    pub particles_per_wave: usize,
    pub wave_time: f32,
    pub delay: f32,
    pub repeat: Option<usize>,
    pub rotation: f32,
    pub gravity: Gravity,
    pub sort: Option<SortBy>,
    // pub blend_mode: todo!(),
    pub attributes: Attributes,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            id: "my_emitter".to_string(),
            kind: EmitterKind::Point,
            offset: Vec2::ZERO,
            index: 0.0,
            particles_per_wave: 2,
            wave_time: 1.0,
            delay: 0.0,
            repeat: None,
            rotation: 0.0,
            gravity: Gravity {
                angle: 0.0,
                amount: 0.0,
            },
            sort: None,
            attributes: Attributes {
                // textures: vec![],
                lifetime: Value::Range { min: 0.2, max: 2.0 },
                scale_x: Attr {
                    initial: Value::Fixed(1.0),
                    behavior: None,
                },
                scale_y: Attr {
                    initial: Value::Fixed(1.0),
                    behavior: None,
                },
                rgb: ColorAttr {
                    initial: Value::Fixed(vec3(1.0, 1.0, 1.0)),
                    behavior: None,
                },
                alpha: Attr {
                    initial: Value::Fixed(1.0),
                    behavior: None,
                },
                speed: Attr {
                    initial: Value::Range {
                        min: 60.0,
                        max: 90.0,
                    },
                    behavior: None,
                },
                rotation: Attr {
                    initial: Value::Range {
                        min: 0.0,
                        max: 360f32.to_radians(),
                    },
                    behavior: None,
                },
                angle: Attr {
                    initial: Value::Range {
                        min: 0.0,
                        max: 360f32.to_radians(),
                    },
                    behavior: None,
                },
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attributes {
    // pub textures: Vec<gfx::Texture>,
    pub lifetime: Value<f32>,
    pub scale_x: Attr<f32>,
    pub scale_y: Attr<f32>,
    pub rgb: ColorAttr,
    pub alpha: Attr<f32>,
    pub speed: Attr<f32>,
    pub rotation: Attr<f32>,
    pub angle: Attr<f32>,
}

#[derive(Debug, Default, Clone)]
pub struct ParticleEmitter {
    pub spawn_accumulator: f32,
    pub time: f32,
    pub delay: f32,
    pub ended: bool,
    pub repeats: usize,
    pub particles: Vec<Particle>,
}

impl ParticleEmitter {
    pub fn reset(&mut self) {
        self.spawn_accumulator = 0.0;
        self.time = 0.0;
        self.delay = 0.0;
        self.repeats = 0;
    }

    pub fn clear(&mut self) {
        self.reset();
        self.particles.clear();
    }
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub life: f32,
    pub pos: Vec2,
    pub initial_scale: Vec2,
    pub scale: Vec2,
    pub initial_color: Color,
    pub color: Color,
    pub initial_speed: f32,
    pub speed: f32,
    pub initial_angle: f32,
    pub angle: f32,
    pub initial_rotation: f32,
    pub rotation: f32,
}

impl Particle {
    fn new(attrs: &Attributes, pos: Vec2) -> Self {
        let life = attrs.lifetime.val();
        let scale = vec2(attrs.scale_x.init(), attrs.scale_y.init());
        let rgb = attrs.rgb.init();
        let color = Color::rgba(rgb.x, rgb.y, rgb.z, attrs.alpha.init());
        let speed = attrs.speed.init();
        let angle = attrs.angle.init();
        let rotation = attrs.rotation.init();

        Self {
            life,
            pos,
            initial_scale: scale,
            scale,
            initial_color: color,
            color,
            initial_speed: speed,
            speed,
            initial_angle: angle,
            angle,
            initial_rotation: rotation,
            rotation,
        }
    }
}

#[derive(Component, Debug, Default, Clone)]
pub struct ParticleFx {
    pub id: String,
    pub pos: Vec2,
    pub emitters: Vec<ParticleEmitter>,
    pub spawning: bool,
}

impl ParticleFx {
    pub fn reset(&mut self) {
        self.emitters.iter_mut().for_each(|emitter| {
            emitter.reset();
        });
    }

    pub fn clear(&mut self) {
        self.emitters.iter_mut().for_each(|emitter| {
            emitter.clear();
        });
    }
}

fn update_system(mut particles: Query<&mut ParticleFx>, time: Res<Time>, configs: Res<Particles>) {
    let dt = time.delta_f32();
    particles.iter_mut().for_each(|mut p| {
        let Some(config) = configs.get(&p.id) else {
            log::warn!("Invalid particle id: {}", &p.id);
            return;
        };

        let origin_position = p.pos;
        let spawning = p.spawning;
        config
            .emitters
            .iter()
            .zip(p.emitters.iter_mut())
            .for_each(|(cfg, emitter)| {
                let attrs = &cfg.attributes;

                let spawn_rate = cfg.particles_per_wave as f32 / cfg.wave_time;
                emitter.spawn_accumulator += spawn_rate * dt;
                let to_spawn = emitter.spawn_accumulator.floor() as usize;
                emitter.spawn_accumulator -= to_spawn as f32;
                emitter.particles.retain(|p| p.life > 0.0);

                let running = !emitter.ended;
                let in_delay = emitter.delay > 0.0;
                let can_spawn = spawning && running && !in_delay;
                if can_spawn {
                    for _ in 0..to_spawn {
                        let center = origin_position + cfg.offset;
                        let local_pos = match cfg.kind {
                            EmitterKind::Point => Vec2::ZERO,
                            EmitterKind::Rect(size) => {
                                let half = size * 0.5;
                                let min = -half;
                                let max = half;
                                vec2(random::range(min.x..max.x), random::range(min.y..max.y))
                            }
                            EmitterKind::Circle(radius) => {
                                let theta = random::range(0.0..TAU);
                                let r = radius * random::range(0.0f32..1.0).sqrt();
                                vec2(r * theta.cos(), r * theta.sin())
                            }
                            EmitterKind::Ring { radius, width } => {
                                let half = width * 0.5;
                                let r_min = (radius - half).max(0.0);
                                let r_max = radius + half;
                                let u = random::range(r_min * r_min..r_max * r_max);
                                let r = u.sqrt();
                                let theta = random::range(0.0..TAU);
                                vec2(r * theta.cos(), r * theta.sin())
                            }
                        };

                        let local_rotated = if cfg.rotation != 0.0 {
                            let (s, c) = cfg.rotation.sin_cos();
                            vec2(
                                local_pos.x * c - local_pos.y * s,
                                local_pos.x * s + local_pos.y * c,
                            )
                        } else {
                            local_pos
                        };

                        let p = Particle::new(attrs, center + local_rotated);
                        emitter.particles.push(p);
                    }
                }

                emitter.particles.iter_mut().for_each(|p| {
                    let progress = 1.0 - (p.life / attrs.lifetime.max());
                    p.scale.x = attrs
                        .scale_x
                        .apply(p.initial_scale.x, p.scale.x, dt, progress);
                    p.scale.y = attrs
                        .scale_y
                        .apply(p.initial_scale.y, p.scale.y, dt, progress);

                    let rgb = attrs.rgb.apply(
                        p.initial_color.to_rgb().into(),
                        p.color.to_rgb().into(),
                        dt,
                        progress,
                    );
                    p.color.r = rgb.x.clamp(0.0, 1.0);
                    p.color.g = rgb.y.clamp(0.0, 1.0);
                    p.color.b = rgb.z.clamp(0.0, 1.0);
                    p.color.a = attrs
                        .alpha
                        .apply(p.initial_color.a, p.color.a, dt, progress)
                        .clamp(0.0, 1.0);

                    p.speed = attrs.speed.apply(p.initial_speed, p.speed, dt, progress);
                    p.rotation = attrs
                        .rotation
                        .apply(p.initial_rotation, p.rotation, dt, progress);
                    p.angle = attrs.angle.apply(p.initial_angle, p.angle, dt, progress);
                    p.pos += Vec2::from_angle(p.angle) * p.speed * dt;

                    p.life -= dt;
                });

                if emitter.delay > 0.0 {
                    emitter.delay -= dt;
                } else {
                    emitter.time += dt;
                    if emitter.time >= cfg.wave_time {
                        emitter.delay = cfg.delay;
                        emitter.time = 0.0;
                        if let Some(repeat) = cfg.repeat {
                            emitter.repeats += 1;
                            if emitter.repeats >= repeat {
                                emitter.ended = true;
                            }
                        }
                    }
                }
            });
    });
}

pub trait ParticlesDraw2DExt {
    fn particle(&mut self, particle: &ParticleFx);
}

impl ParticlesDraw2DExt for Draw2D {
    fn particle(&mut self, fx: &ParticleFx) {
        fx.emitters.iter().for_each(|emitter| {
            emitter.particles.iter().for_each(|p| {
                self.rect(Vec2::ZERO, Vec2::splat(10.0))
                    .origin(Vec2::splat(0.5))
                    .translate(p.pos)
                    .scale(p.scale)
                    .rotation(p.rotation)
                    .color(p.color);
            });
        });
    }
}
