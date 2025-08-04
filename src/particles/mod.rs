use std::{
    array,
    cmp::Ordering,
    f32::consts::TAU,
    ops::{Add, Mul},
};

use corelib::{
    gfx::TextureFilter,
    math::{Mat3, Rect, Vec3, vec3},
};
use draw::{Draw2D, Sprite, create_sprite};
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

#[derive(Event, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ParticleEndEvent {
    entity: Entity,
}

#[derive(Debug, Clone, Copy)]
pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn apply(&self, app: &mut App) {
        app.add_event::<ParticleEndEvent>()
            .add_resource(Particles::default())
            .add_systems(
                OnSetup,
                load_default_particles_system.in_set(ParticlesSysSet),
            )
            .add_systems(OnUpdate, update_system.in_set(ParticlesSysSet))
            .configure_sets(OnUpdate, ParticlesSysSet);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParticleSprite {
    Id(String),
    #[serde(skip)]
    Sprite {
        id: Option<String>,
        sprite: Sprite,
    },
}

#[derive(Resource, Default, Clone)]
pub struct Particles {
    sprites: FxHashMap<String, Sprite>,
    config: FxHashMap<String, ParticleFxConfig>,
}

impl Particles {
    #[inline]
    pub fn iter_sprites(&self) -> impl Iterator<Item = (&String, &Sprite)> {
        self.sprites.iter()
    }

    #[inline]
    pub fn add_sprite(&mut self, id: &str, sprite: Sprite) {
        // find emitter that could use this sprite and add it to them
        for cfg in self.config.values_mut() {
            for emitter in &mut cfg.emitters {
                for ps in &mut emitter.sprites {
                    if let ParticleSprite::Id(ps_id) = ps {
                        if ps_id == id {
                            *ps = ParticleSprite::Sprite {
                                id: Some(id.to_string()),
                                sprite: sprite.clone(),
                            };
                        }
                    }
                }
            }
        }

        self.sprites.insert(id.to_string(), sprite);
    }

    #[inline]
    pub fn add_config(&mut self, id: &str, mut cfg: ParticleFxConfig) {
        // update sprite references in the config before storing it
        for emitter in &mut cfg.emitters {
            for ps in &mut emitter.sprites {
                if let ParticleSprite::Id(ps_id) = ps {
                    if let Some(sprite) = self.sprites.get(ps_id) {
                        *ps = ParticleSprite::Sprite {
                            id: Some(ps_id.to_string()),
                            sprite: sprite.clone(),
                        };
                    }
                }
            }
        }

        self.config.insert(id.to_string(), cfg);
    }

    #[inline]
    pub fn get_config(&self, id: &str) -> Option<&ParticleFxConfig> {
        self.config.get(id)
    }

    #[inline]
    pub fn get_config_mut(&mut self, id: &str) -> Option<&mut ParticleFxConfig> {
        self.config.get_mut(id)
    }

    #[inline]
    pub fn create_component(&self, id: &str, pos: Vec2) -> Option<ParticleFx> {
        let cfg = self.config.get(id)?;

        let emitters = cfg
            .emitters
            .iter()
            .map(|emitter_config| {
                let sprites: Vec<Sprite> = emitter_config
                    .sprites
                    .iter()
                    .filter_map(|ps| match ps {
                        ParticleSprite::Sprite { sprite, .. } => Some(sprite.clone()),
                        _ => None,
                    })
                    .collect();

                ParticleEmitter {
                    def: emitter_config.def.clone(),
                    sprites,
                    spawn_accumulator: 0.0,
                    time: 0.0,
                    delay: 0.0,
                    ended: false,
                    repeats: 0,
                    particles: vec![],
                }
            })
            .collect();

        Some(ParticleFx {
            id: id.to_string(),
            pos,
            emitters,
            spawning: true,
            disabled: false,
            is_local: cfg.is_local,
            ended: false,
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

impl<T> Default for Value<T>
where
    T: Default + Interpolable,
{
    fn default() -> Self {
        Self::Fixed(Default::default())
    }
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

impl Default for Attr<f32> {
    fn default() -> Self {
        Self {
            initial: Default::default(),
            behavior: Default::default(),
        }
    }
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
    pub is_local: bool,
    pub emitters: Vec<EmitterConfig>,
}

impl Default for ParticleFxConfig {
    fn default() -> Self {
        Self {
            is_local: false,
            emitters: vec![EmitterConfig::default()],
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, AsRefStr, Serialize, Deserialize)]
pub enum EmitterShape {
    #[default]
    Point,
    Rect(Vec2),
    Circle(f32),
    Ring {
        radius: f32,
        width: f32,
    },
    Radial {
        count: usize,
        radius: f32,
    },
}

#[derive(Default, Debug, Clone, Copy, PartialEq, AsRefStr, EnumIter, Serialize, Deserialize)]
pub enum SortBy {
    #[default]
    SpawnOnTop,
    SpawnOnBottom,
    TopDownSort,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Gravity {
    pub angle: f32,
    pub amount: f32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EmitterDef {
    pub kind: EmitterShape,
    pub offset: Vec2,
    pub particles_per_wave: usize,
    pub wave_time: f32,
    pub delay: f32,
    pub repeat: Option<usize>,
    pub rotation: f32,
    pub gravity: Gravity,
    pub sort: SortBy,
    pub attributes: Attributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitterConfig {
    pub id: String,
    pub def: EmitterDef,
    // pub blend_mode: todo!(),
    pub sprites: Vec<ParticleSprite>,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            id: "my_emitter".to_string(),
            def: EmitterDef {
                kind: EmitterShape::Point,
                offset: Vec2::ZERO,
                particles_per_wave: 2,
                wave_time: 1.0,
                delay: 0.0,
                repeat: None,
                rotation: 0.0,
                gravity: Gravity {
                    angle: 0.0,
                    amount: 0.0,
                },
                sort: SortBy::default(),
                attributes: Attributes {
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
                    rotation: RotationAttr {
                        lock_to_angle: false,
                        attr: Attr {
                            initial: Value::Range {
                                min: 0.0,
                                max: 360f32.to_radians(),
                            },
                            behavior: None,
                        },
                    },
                    direction: Attr {
                        initial: Value::Range {
                            min: 0.0,
                            max: 360f32.to_radians(),
                        },
                        behavior: None,
                    },
                },
            },
            sprites: vec![],
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RotationAttr {
    pub lock_to_angle: bool,
    pub attr: Attr<f32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Attributes {
    pub lifetime: Value<f32>,
    pub scale_x: Attr<f32>,
    pub scale_y: Attr<f32>,
    pub rgb: ColorAttr,
    pub alpha: Attr<f32>,
    pub speed: Attr<f32>,
    pub rotation: RotationAttr,
    pub direction: Attr<f32>,
}

#[derive(Debug, Default, Clone)]
pub struct ParticleEmitter {
    pub def: EmitterDef,
    pub spawn_accumulator: f32,
    pub time: f32,
    pub delay: f32,
    pub ended: bool,
    pub repeats: usize,
    pub sprites: Vec<Sprite>,
    pub particles: Vec<Particle>,
}

impl ParticleEmitter {
    pub fn reset(&mut self) {
        self.spawn_accumulator = 0.0;
        self.time = 0.0;
        self.delay = 0.0;
        self.repeats = 0;
        self.ended = false;
    }

    pub fn clear(&mut self) {
        self.reset();
        self.particles.clear();
    }
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub sprite: Option<usize>,
    pub spawn_time: f32,
    pub total_life: f32,
    pub life: f32,
    pub pos: Vec2,
    pub initial_scale: Vec2,
    pub scale: Vec2,
    pub initial_color: Color,
    pub color: Color,
    pub initial_speed: f32,
    pub speed: f32,
    pub initial_angle: f32,
    pub direction: f32,
    pub initial_rotation: f32,
    pub rotation: f32,
}

impl Particle {
    fn new(attrs: &Attributes, pos: Vec2, spawn_time: f32, sprite_idx: Option<usize>) -> Self {
        let total_life = attrs.lifetime.val();
        let scale = vec2(attrs.scale_x.init(), attrs.scale_y.init());
        let rgb = attrs.rgb.init();
        let color = Color::rgba(rgb.x, rgb.y, rgb.z, attrs.alpha.init());
        let speed = attrs.speed.init();
        let angle = attrs.direction.init();
        let rotation = attrs.rotation.attr.init();

        Self {
            sprite: sprite_idx,
            spawn_time,
            total_life,
            life: 0.0,
            pos,
            initial_scale: scale,
            scale,
            initial_color: color,
            color,
            initial_speed: speed,
            speed,
            initial_angle: angle,
            direction: angle,
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
    pub disabled: bool,
    pub is_local: bool,
    pub ended: bool,
}

impl ParticleFx {
    pub fn disable(&mut self) {
        self.disabled = true;
    }

    pub fn enable(&mut self) {
        self.disabled = false;
    }

    pub fn reset(&mut self) {
        self.emitters.iter_mut().for_each(|emitter| {
            emitter.reset();
        });
        self.ended = false;
    }

    pub fn clear(&mut self) {
        self.emitters.iter_mut().for_each(|emitter| {
            emitter.clear();
        });
    }

    pub fn bounds(&self) -> Rect {
        if self.emitters.is_empty() {
            return Rect::new(self.pos, Vec2::ONE);
        }

        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);

        for emitter in &self.emitters {
            let offset = emitter.def.offset;
            let (shape_min, shape_max) = match emitter.def.kind {
                EmitterShape::Point => (Vec2::ZERO, Vec2::ZERO),
                EmitterShape::Rect(size) => {
                    let half = size * 0.5;
                    (-half, half)
                }
                EmitterShape::Circle(r) => {
                    let v = Vec2::splat(r);
                    (-v, v)
                }
                EmitterShape::Ring { radius, width } => {
                    let outer = radius + width * 0.5;
                    let v = Vec2::splat(outer);
                    (-v, v)
                }
                EmitterShape::Radial { radius, .. } => {
                    let v = Vec2::splat(radius);
                    (-v, v)
                }
            };

            let local_min = self.pos + offset + shape_min;
            let local_max = self.pos + offset + shape_max;

            min.x = min.x.min(local_min.x);
            min.y = min.y.min(local_min.y);
            max.x = max.x.max(local_max.x);
            max.y = max.y.max(local_max.y);
        }

        Rect::new(min, max - min)
    }
}

fn load_default_particles_system(mut config: ResMut<Particles>) {
    let spritesheet = create_sprite()
        .from_image(include_bytes!("./particles.png"))
        .with_label("Particles Default Spritesheet")
        .with_min_filter(TextureFilter::Nearest)
        .with_mag_filter(TextureFilter::Nearest)
        .build()
        .unwrap();

    let cols = 16;
    let particles = 64;
    let size = Vec2::splat(16.0);

    (0..particles).for_each(|n| {
        let pos = vec2((n % cols) as f32 * size.x, (n / cols) as f32 * size.y);
        let sprite = spritesheet.clone_with_frame(Rect::new(pos, size));
        config.add_sprite(&format!("particle_{n}"), sprite);
    });
}

fn update_system(
    mut particles: Query<(Entity, &mut ParticleFx)>,
    time: Res<Time>,
    mut evt: EventWriter<ParticleEndEvent>,
) {
    let dt = time.delta_f32();
    particles
        .iter_mut()
        .filter(|(_, p)| !(p.disabled || p.ended))
        .for_each(|(entity, mut p)| {
            let origin_position = if p.is_local { Vec2::ZERO } else { p.pos };
            let spawning = p.spawning;
            p.emitters.iter_mut().for_each(|emitter| {
                let spawn_rate = emitter.def.particles_per_wave as f32 / emitter.def.wave_time;
                emitter.spawn_accumulator += spawn_rate * dt;
                let to_spawn = emitter.spawn_accumulator.floor() as usize;
                emitter.spawn_accumulator -= to_spawn as f32;
                emitter.particles.retain(|p| p.life < p.total_life);

                let running = !emitter.ended;
                let in_delay = emitter.delay > 0.0;
                let can_spawn = spawning && running && !in_delay;
                if can_spawn {
                    for _ in 0..to_spawn {
                        let mut skip_spawn = false;
                        let center = origin_position + emitter.def.offset;
                        let local_pos = match emitter.def.kind {
                            EmitterShape::Point => Vec2::ZERO,
                            EmitterShape::Rect(size) => {
                                let half = size * 0.5;
                                let min = -half;
                                let max = half;
                                vec2(random::range(min.x..max.x), random::range(min.y..max.y))
                            }
                            EmitterShape::Circle(radius) => {
                                let theta = random::range(0.0..TAU);
                                let r = radius * random::range(0.0f32..1.0).sqrt();
                                vec2(r * theta.cos(), r * theta.sin())
                            }
                            EmitterShape::Ring { radius, width } => {
                                let half = width * 0.5;
                                let r_min = (radius - half).max(0.0);
                                let r_max = radius + half;
                                let u = random::range(r_min * r_min..r_max * r_max);
                                let r = u.sqrt();
                                let theta = random::range(0.0..TAU);
                                vec2(r * theta.cos(), r * theta.sin())
                            }
                            EmitterShape::Radial { count, radius } => {
                                for i in 0..count {
                                    let angle = TAU * (i as f32) / (count as f32);
                                    let local_pos = Vec2::from_angle(angle) * radius;
                                    spawn_particle(center, local_pos, emitter, time.elapsed_f32());
                                }

                                skip_spawn = true;
                                Vec2::ZERO
                            }
                        };

                        if !skip_spawn {
                            spawn_particle(center, local_pos, emitter, time.elapsed_f32());
                        }
                    }
                }

                emitter.particles.iter_mut().for_each(|p| {
                    // The progress is defined based on the maximum lifetime of the particle
                    let progress = p.life / emitter.def.attributes.lifetime.max();

                    // The scale define the size, there is no size in pixel but we use
                    // a scale based on the particle shape or texture
                    p.scale.x = emitter.def.attributes.scale_x.apply(
                        p.initial_scale.x,
                        p.scale.x,
                        dt,
                        progress,
                    );
                    p.scale.y = emitter.def.attributes.scale_y.apply(
                        p.initial_scale.y,
                        p.scale.y,
                        dt,
                        progress,
                    );

                    // The color is defined by channels with alpha as a different property
                    let rgb = emitter.def.attributes.rgb.apply(
                        p.initial_color.to_rgb().into(),
                        p.color.to_rgb().into(),
                        dt,
                        progress,
                    );
                    p.color.r = rgb.x.clamp(0.0, 1.0);
                    p.color.g = rgb.y.clamp(0.0, 1.0);
                    p.color.b = rgb.z.clamp(0.0, 1.0);
                    p.color.a = emitter
                        .def
                        .attributes
                        .alpha
                        .apply(p.initial_color.a, p.color.a, dt, progress)
                        .clamp(0.0, 1.0);

                    // Now we set the movement of the particle based in different attributes
                    let speed =
                        emitter
                            .def
                            .attributes
                            .speed
                            .apply(p.initial_speed, p.speed, dt, progress);
                    let direction = emitter.def.attributes.direction.apply(
                        p.initial_angle,
                        p.direction,
                        dt,
                        progress,
                    );

                    let gravity =
                        Vec2::from_angle(emitter.def.gravity.angle) * emitter.def.gravity.amount;
                    let vel = Vec2::from_angle(direction) * speed + gravity * dt;
                    p.speed = vel.length();
                    p.direction = vel.to_angle();

                    // The rotation define the angle of the particle from its own center
                    p.rotation = if emitter.def.attributes.rotation.lock_to_angle {
                        p.direction + p.initial_rotation
                    } else {
                        emitter.def.attributes.rotation.attr.apply(
                            p.initial_rotation,
                            p.rotation,
                            dt,
                            progress,
                        )
                    };

                    // and then apply all to get the current position
                    p.pos += vel * dt;

                    p.life = (p.life + dt).min(p.total_life);
                });

                match emitter.def.sort {
                    SortBy::SpawnOnBottom => {
                        if to_spawn > 0 {
                            emitter.particles.sort_unstable_by(|a, b| {
                                b.spawn_time
                                    .partial_cmp(&a.spawn_time)
                                    .unwrap_or(Ordering::Equal)
                            });
                        }
                    }
                    SortBy::TopDownSort => {
                        emitter.particles.sort_unstable_by(|a, b| {
                            a.pos.y.partial_cmp(&b.pos.y).unwrap_or(Ordering::Equal)
                        });
                    }
                    _ => {}
                }

                if emitter.delay > 0.0 {
                    emitter.delay -= dt;
                } else {
                    emitter.time += dt;
                    if emitter.time >= emitter.def.wave_time {
                        emitter.delay = emitter.def.delay;
                        emitter.time = 0.0;
                        if let Some(repeat) = emitter.def.repeat {
                            emitter.repeats += 1;
                            if emitter.repeats >= repeat {
                                emitter.ended = true;
                            }
                        }
                    }
                }
            });

            // we need to fire the ended event when all emitter are done but only when the last
            // emitter ends
            let all_done = !p.ended
                && p.emitters
                    .iter()
                    .all(|emitter| emitter.ended && emitter.particles.is_empty());
            if all_done {
                p.spawning = false;
                p.ended = true;
                evt.send(ParticleEndEvent { entity });

                println!("send end event");
            }
        });
}

pub trait ParticlesDraw2DExt {
    fn particle(&mut self, particle: &ParticleFx);
}

impl ParticlesDraw2DExt for Draw2D {
    fn particle(&mut self, fx: &ParticleFx) {
        let is_local = fx.is_local;
        if is_local {
            self.push_matrix(Mat3::from_translation(fx.pos));
        }

        fx.emitters.iter().for_each(|emitter| {
            emitter.particles.iter().for_each(|p| {
                match p.sprite.and_then(|idx| emitter.sprites.get(idx)) {
                    Some(sprite) => {
                        self.image(sprite)
                            .origin(Vec2::splat(0.5))
                            .translate(p.pos)
                            .scale(p.scale)
                            .rotation(p.rotation)
                            .color(p.color);
                    }
                    None => {
                        self.rect(Vec2::ZERO, Vec2::splat(10.0))
                            .origin(Vec2::splat(0.5))
                            .translate(p.pos)
                            .scale(p.scale)
                            .rotation(p.rotation)
                            .color(p.color);
                    }
                }
            });
        });

        if is_local {
            self.pop_matrix();
        }
    }
}

fn spawn_particle(center: Vec2, local_pos: Vec2, emitter: &mut ParticleEmitter, spawn_time: f32) {
    let local_rotated = if emitter.def.rotation != 0.0 {
        let (s, c) = emitter.def.rotation.sin_cos();
        vec2(
            local_pos.x * c - local_pos.y * s,
            local_pos.x * s + local_pos.y * c,
        )
    } else {
        local_pos
    };

    let sprite_idx = if emitter.sprites.is_empty() {
        None
    } else {
        Some(random::range(0..emitter.sprites.len()))
    };

    let p = Particle::new(
        &emitter.def.attributes,
        center + local_rotated,
        spawn_time,
        sprite_idx,
    );
    emitter.particles.push(p);
}
