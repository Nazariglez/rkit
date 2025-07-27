use draw::create_draw_2d;
use rkit::{
    ecs::prelude::*,
    egui::{EguiContext, EguiPlugin},
    gfx::{self, Color},
    particles::*,
};

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(EguiPlugin::default())
        .add_plugin(ParticlesPlugin)
        .add_plugin(WindowConfigPlugin::default().maximized(true))
        .add_systems(OnSetup, setup_system)
        .add_systems(OnUpdate, update_system.before(ParticlesSysSet))
        .add_systems(OnRender, draw_system)
        .run()
}

fn setup_system(mut cmds: Commands, mut configs: ResMut<Particles>, window: Res<Window>) {
    configs.insert("my_fx".to_string(), ParticleFxConfig::default());
    cmds.spawn(configs.create_fx("my_fx", window.size() * 0.5).unwrap());
}

fn update_system(mut fx: Single<&mut ParticleFx>, mouse: Res<Mouse>, ctx: Res<EguiContext>) {
    fx.spawning = true;

    // FIXME: this is not working right
    if ctx.wants_pointer() {
        return;
    }

    if mouse.is_down(MouseButton::Left) {
        // fx.pos = mouse.position();
    }
}

fn draw_system(
    fx: Single<&ParticleFx>,
    mut ctx: ResMut<EguiContext>,
    mut configs: ResMut<Particles>,
) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    let fx = fx.into_inner();
    let fx_id = fx.id.clone();
    draw.particle(fx);

    gfx::render_to_frame(&draw).unwrap();

    let edraw = ctx.run(|ctx| {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Particle Controls");
                let cfg: &mut ParticleFxConfig =
                    configs.get_mut(&fx_id).expect("config must exist");

                for (i, emitter) in cfg.emitters.iter_mut().enumerate() {
                    ui.collapsing(format!("Emitter #{i}"), |ui| {
                        ui.separator();
                        ui.label("Attributes:");

                        fn attr_ui(
                            ui: &mut egui::Ui,
                            name: &str,
                            attr: &mut Attr<f32>,
                            init_min: f32,
                            init_max: f32,
                        ) {
                            ui.collapsing(name, |ui| {
                                // INITIAL VALUE UI
                                ui.label("Initial:");
                                match &mut attr.initial {
                                    Value::Fixed(v) => {
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::Slider::new(v, init_min..=init_max)
                                                    .text("fixed"),
                                            );
                                        });
                                    }
                                    Value::Range { min, max } => {
                                        ui.horizontal(|ui| {
                                            ui.label("min:");
                                            ui.add(
                                                egui::DragValue::new(min)
                                                    .clamp_range(init_min..=init_max),
                                            );
                                            ui.label("max:");
                                            ui.add(
                                                egui::DragValue::new(max)
                                                    .clamp_range(init_min..=init_max),
                                            );
                                        });
                                    }
                                }

                                ui.separator();
                                ui.label("Behavior:");
                                let mut mode = match &attr.behavior {
                                    None => 0,
                                    Some(Behavior::Fixed { .. }) => 1,
                                    Some(Behavior::Increment(_)) => 2,
                                };
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut mode, 0, "None");
                                    ui.selectable_value(&mut mode, 1, "Fixed");
                                    ui.selectable_value(&mut mode, 2, "Inc");
                                });
                                match mode {
                                    0 => attr.behavior = None,
                                    1 if !matches!(attr.behavior, Some(Behavior::Fixed { .. })) => {
                                        let s = attr.initial.min();
                                        let e = attr.initial.max();
                                        attr.behavior = Some(Behavior::Fixed {
                                            start: s,
                                            end: e,
                                            curve: Curve::Linear,
                                        });
                                    }
                                    2 if !matches!(attr.behavior, Some(Behavior::Increment(_))) => {
                                        attr.behavior = Some(Behavior::Increment(0.0));
                                    }
                                    _ => {}
                                }
                                if let Some(beh) = &mut attr.behavior {
                                    match beh {
                                        Behavior::Fixed { start, end, curve } => {
                                            ui.horizontal(|ui| {
                                                ui.label("Start:");
                                                ui.add(egui::DragValue::new(start).speed(0.1));
                                                ui.label("End:");
                                                ui.add(egui::DragValue::new(end).speed(0.1));
                                            });
                                            egui::ComboBox::from_label("Curve")
                                                .selected_text(format!("{curve:?}"))
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(
                                                        curve,
                                                        Curve::Linear,
                                                        "Linear",
                                                    );
                                                    ui.selectable_value(
                                                        curve,
                                                        Curve::Custom(vec![]),
                                                        "Custom",
                                                    );
                                                });
                                        }
                                        Behavior::Increment(delta) => {
                                            ui.horizontal(|ui| {
                                                ui.label("Per sec:");
                                                ui.add(egui::DragValue::new(delta).speed(0.1));
                                            });
                                        }
                                    }
                                }
                            });
                        }

                        attr_ui(ui, "Scale X", &mut emitter.attributes.scale_x, 0.0, 5.0);
                        attr_ui(ui, "Scale Y", &mut emitter.attributes.scale_y, 0.0, 5.0);
                        attr_ui(ui, "Red", &mut emitter.attributes.red, 0.0, 1.0);
                        attr_ui(ui, "Green", &mut emitter.attributes.green, 0.0, 1.0);
                        attr_ui(ui, "Blue", &mut emitter.attributes.blue, 0.0, 1.0);
                        attr_ui(ui, "Alpha", &mut emitter.attributes.alpha, 0.0, 1.0);
                    });
                }
            });
    });

    gfx::render_to_frame(&edraw).unwrap();
}
