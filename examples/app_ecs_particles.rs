use rkit::{
    draw::create_draw_2d,
    gfx::{self, Color},
    math::{Vec2, vec2},
    particles::*,
    prelude::*,
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(ParticlesPlugin)
        .add_systems(OnSetup, setup_system)
        .add_systems(OnUpdate, update_system.before(ParticlesSysSet))
        .add_systems(OnRender, draw_system)
        .run()
}

fn setup_system(mut cmds: Commands, mut configs: ResMut<Particles>, window: Res<Window>) {
    configs.insert(
        "basic".to_string(),
        ParticleFxConfig {
            emitters: vec![EmitterConfig {
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
                repeat: None,
                delay: 0.0,
                sort: None,
                attributes: Attributes {
                    lifetime: Value::Range { min: 1.0, max: 3.0 }, // Particles live 1-3 seconds
                    scale_x: Attr {
                        initial: Value::Fixed(1.0),
                        behavior: Some(Behavior::To {
                            value: 0.0,
                            curve: Curve::Linear,
                        }),
                    },
                    scale_y: Attr {
                        initial: Value::Fixed(1.0),
                        behavior: Some(Behavior::To {
                            value: 0.0,
                            curve: Curve::Linear,
                        }),
                    },
                    red: Attr {
                        initial: Value::Range { min: 0.4, max: 1.0 },
                        behavior: Some(Behavior::To {
                            value: 1.0,
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
                        behavior: Some(Behavior::To {
                            value: 0.0,
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
        },
    );

    configs.insert(
        "burst".to_string(),
        ParticleFxConfig {
            emitters: vec![EmitterConfig {
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
                    lifetime: Value::Range { min: 1.0, max: 3.0 }, // Particles live 1-3 seconds
                    scale_x: Attr {
                        initial: Value::Fixed(1.0),
                        behavior: Some(Behavior::To {
                            value: 0.0,
                            curve: Curve::Linear,
                        }),
                    },
                    scale_y: Attr {
                        initial: Value::Fixed(1.0),
                        behavior: Some(Behavior::To {
                            value: 0.0,
                            curve: Curve::Linear,
                        }),
                    },
                    red: Attr {
                        initial: Value::Range { min: 0.4, max: 1.0 },
                        behavior: Some(Behavior::To {
                            value: 1.0,
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
                        behavior: Some(Behavior::To {
                            value: 0.0,
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
        },
    );

    cmds.spawn(
        configs
            .create_component("basic", window.size() * 0.5)
            .unwrap(),
    );
}

fn update_system(mouse: Res<Mouse>, mut particles: Query<&mut ParticleFx>) {
    particles.iter_mut().for_each(|mut p| {
        p.pos = mouse.position();
        p.spawning = mouse.is_down(MouseButton::Left);
    });
}

fn draw_system(particles: Query<&ParticleFx>, time: Res<Time>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    particles.iter().for_each(|fx| {
        draw.particle(fx);
    });

    draw.text(&format!("{:.2}", time.fps()));
    gfx::render_to_frame(&draw).unwrap();
}
