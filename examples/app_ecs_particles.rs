use rkit::{
    draw::create_draw_2d,
    gfx::{self, Color},
    math::{Mat3, Vec2, vec2},
    particles::*,
    prelude::*,
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_systems(OnSetup, setup_system)
        .add_systems(OnUpdate, update_system)
        .add_systems(OnRender, draw_system)
        .run()
}

// TODO: behavior range should start from the initial value,
#[derive(Debug, Component, Clone, Copy)]
pub struct Pos(Vec2);

#[derive(Resource)]
pub struct ParticleConfigs {
    pub basic: Effect,
    pub burst: Effect,
}

#[derive(Debug, Default)]
pub struct ParticleEmitter {
    pub spawn_accumulator: f32,
    pub time: f32,
    pub delay: f32,
    pub enabled: bool,
    pub repeats: usize,
    pub particles: Vec<Particle>,
}

#[derive(Debug, Default)]
pub struct Particle {
    pub life: f32,
    pub pos: Vec2,
    pub scale: Vec2,
    pub color: Color,
    pub speed: f32,
    pub angle: f32,
    pub rotation: f32,
}

#[derive(Component)]
pub struct MyParticle {
    particles: Vec<ParticleEmitter>,
}

fn setup_system(mut cmds: Commands) {
    let basic = Effect {
        pos: vec2(400.0, 300.0),
        emitters: vec![Emitter {
            id: "simple".to_string(),
            kind: EmitterKind::Square(vec2(100.0, 100.0)),
            offset: Vec2::ZERO,
            index: 0.0,
            particles_per_wave: 10000,
            wave_time: 1.0,
            gravity: Gravity {
                angle: 0.0,
                amount: 0.0,
            },
            repeat: Some(2),
            delay: 1.0,
            sort: None,
            attributes: Attributes {
                textures: vec![], // No textures for now, will render as colored squares
                lifetime: Value::Range { min: 1.0, max: 3.0 }, // Particles live 1-3 seconds
                scale_x: Attr {
                    initial: Value::Fixed(1.0),
                    behavior: Some(Behavior::Fixed {
                        start: 1.0,
                        end: 0.0,
                        curve: Curve::Linear,
                    }),
                },
                scale_y: Attr {
                    initial: Value::Fixed(1.0),
                    behavior: Some(Behavior::Fixed {
                        start: 1.0,
                        end: 0.0,
                        curve: Curve::Linear,
                    }),
                },
                red: Attr {
                    initial: Value::Range { min: 0.4, max: 1.0 },
                    behavior: Some(Behavior::Fixed {
                        start: 0.4,
                        end: 1.0,
                        curve: Curve::Linear,
                    }),
                },
                blue: Attr {
                    initial: Value::Range { min: 0.3, max: 0.4 },
                    behavior: None,
                },
                green: Attr {
                    initial: Value::Range { min: 0.1, max: 0.2 },
                    behavior: None,
                },
                alpha: Attr {
                    initial: Value::Fixed(1.0),
                    behavior: Some(Behavior::Fixed {
                        start: 1.0,
                        end: 0.0,
                        curve: Curve::Linear,
                    }),
                },
                speed: Attr {
                    initial: Value::Range {
                        min: 100.0,
                        max: 450.0,
                    },
                    behavior: Some(Behavior::Increment(-90.0)), // Slow down over time
                },
                rotation: Attr {
                    initial: Value::Range {
                        min: 0.0,
                        max: 360.0f32.to_radians(),
                    },
                    behavior: Some(Behavior::Increment(180.0f32.to_radians())), // Rotate 180 degrees per second
                },
                angle: Attr {
                    initial: Value::Range {
                        min: 0.0,
                        max: 360.0f32.to_radians(),
                    },
                    behavior: Some(Behavior::Increment(90.0f32.to_radians())),
                },
            },
        }],
    };

    let burst_effect = Effect {
        pos: vec2(400.0, 300.0),
        emitters: vec![Emitter {
            id: "simple".to_string(),
            kind: EmitterKind::Square(vec2(100.0, 100.0)),
            offset: Vec2::ZERO,
            index: 0.0,
            particles_per_wave: 1000,
            wave_time: 0.2,
            gravity: Gravity {
                angle: 0.0,
                amount: 0.0,
            },
            repeat: None,
            delay: 0.3,
            sort: None,
            attributes: Attributes {
                textures: vec![], // No textures for now, will render as colored squares
                lifetime: Value::Range { min: 1.0, max: 3.0 }, // Particles live 1-3 seconds
                scale_x: Attr {
                    initial: Value::Fixed(1.0),
                    behavior: Some(Behavior::Fixed {
                        start: 1.0,
                        end: 0.0,
                        curve: Curve::Linear,
                    }),
                },
                scale_y: Attr {
                    initial: Value::Fixed(1.0),
                    behavior: Some(Behavior::Fixed {
                        start: 1.0,
                        end: 0.0,
                        curve: Curve::Linear,
                    }),
                },
                red: Attr {
                    initial: Value::Range { min: 0.4, max: 1.0 },
                    behavior: Some(Behavior::Fixed {
                        start: 0.4,
                        end: 1.0,
                        curve: Curve::Linear,
                    }),
                },
                blue: Attr {
                    initial: Value::Range { min: 0.6, max: 0.8 },
                    behavior: Some(Behavior::Increment(-0.1)),
                },
                green: Attr {
                    initial: Value::Range { min: 0.1, max: 0.2 },
                    behavior: None,
                },
                alpha: Attr {
                    initial: Value::Fixed(1.0),
                    behavior: Some(Behavior::Fixed {
                        start: 1.0,
                        end: 0.0,
                        curve: Curve::Linear,
                    }),
                },
                speed: Attr {
                    initial: Value::Fixed(150.0),
                    behavior: Some(Behavior::Increment(-90.0)), // Slow down over time
                },
                rotation: Attr {
                    initial: Value::Range {
                        min: 0.0,
                        max: 360.0f32.to_radians(),
                    },
                    behavior: Some(Behavior::Increment(180.0f32.to_radians())), // Rotate 180 degrees per second
                },
                angle: Attr {
                    initial: Value::Range {
                        min: 0.0,
                        max: 360.0f32.to_radians(),
                    },
                    behavior: Some(Behavior::Increment(120.0f32.to_radians())),
                },
            },
        }],
    };

    cmds.insert_resource(ParticleConfigs {
        basic,
        burst: burst_effect,
    });

    cmds.spawn((
        MyParticle {
            particles: vec![ParticleEmitter {
                spawn_accumulator: 0.0,
                time: 0.0,
                particles: vec![],
                delay: 0.0,
                repeats: 0,
                enabled: true,
            }],
        },
        Pos(Vec2::ZERO),
    ));
}

fn update_system(
    mut particles: Query<(&mut MyParticle, &mut Pos)>,
    time: Res<Time>,
    configs: Res<ParticleConfigs>,
    mouse: Res<Mouse>,
) {
    let dt = time.delta_f32();
    let config = &configs.burst;
    particles.iter_mut().for_each(|(mut p, mut pos)| {
        pos.0 = mouse.position();

        p.particles.iter_mut().for_each(|emitter| {
            let down = mouse.is_down(MouseButton::Left);
            if emitter.enabled && !down {
                emitter.enabled = false;
            } else if !emitter.enabled && down {
                emitter.enabled = true;
            }
        });

        config
            .emitters
            .iter()
            .zip(p.particles.iter_mut())
            // .filter(|(_cfg, emitter)| emitter.enabled || !emitter.particles.is_empty())
            .for_each(|(cfg, emitter)| {
                let attrs = &cfg.attributes;

                let spawn_rate = cfg.particles_per_wave as f32 / cfg.wave_time;
                emitter.spawn_accumulator += spawn_rate * dt;
                let to_spawn = emitter.spawn_accumulator.floor() as usize;
                emitter.spawn_accumulator -= to_spawn as f32;
                emitter.particles.retain(|p| p.life > 0.0);

                let can_spawn = emitter.enabled && emitter.delay <= 0.0;
                if can_spawn {
                    for _ in 0..to_spawn {
                        let p = Particle {
                            life: attrs.lifetime.val(),
                            pos: pos.0 + cfg.offset, // TODO: random pos based on shape
                            scale: vec2(
                                cfg.attributes.scale_x.init(),
                                cfg.attributes.scale_y.init(),
                            ),
                            color: Color::rgba(
                                attrs.red.init(),
                                attrs.green.init(),
                                attrs.blue.init(),
                                attrs.alpha.init(),
                            ),
                            speed: attrs.speed.init(),
                            angle: attrs.angle.init(),
                            rotation: attrs.rotation.init(),
                        };
                        emitter.particles.push(p);
                    }
                }

                emitter.particles.iter_mut().for_each(|p| {
                    let progress = 1.0 - (p.life / attrs.lifetime.max());
                    p.scale.x = attrs.scale_x.apply(p.scale.x, dt, progress);
                    p.scale.y = attrs.scale_y.apply(p.scale.y, dt, progress);
                    p.color.r = attrs.red.apply(p.color.r, dt, progress);
                    p.color.g = attrs.green.apply(p.color.g, dt, progress);
                    p.color.b = attrs.blue.apply(p.color.b, dt, progress);
                    p.color.a = attrs.alpha.apply(p.color.a, dt, progress);
                    p.speed = attrs.speed.apply(p.speed, dt, progress);
                    p.rotation = attrs.rotation.apply(p.rotation, dt, progress);
                    p.angle = attrs.angle.apply(p.angle, dt, progress);
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
                                emitter.enabled = false;
                            }
                        }
                    }
                }
            });
    });
}

fn draw_system(particles: Query<&MyParticle>, time: Res<Time>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    particles.iter().for_each(|fx| {
        fx.particles.iter().for_each(|emitter| {
            emitter.particles.iter().for_each(|p| {
                draw.rect(Vec2::ZERO, Vec2::splat(10.0))
                    .origin(Vec2::splat(0.5))
                    .translate(p.pos)
                    .scale(p.scale)
                    .rotation(p.rotation)
                    .color(p.color);
            });
        });
    });

    draw.text(&format!("{:.2}", time.fps()));
    gfx::render_to_frame(&draw).unwrap();
}
