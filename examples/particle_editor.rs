use egui::{Align, RichText};
use rkit::{
    draw::create_draw_2d,
    ecs::prelude::*,
    egui::{EguiContext, EguiPlugin, Layout},
    gfx::{self, Color, LinearColor},
    math::Vec2,
    particles::*,
};

#[derive(Resource)]
struct State {
    clear_color: Color,
    selected_emitter: Option<usize>,
    offset_edit_mode: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            clear_color: Color::BLACK,
            selected_emitter: Some(0),
            offset_edit_mode: false,
        }
    }
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(EguiPlugin::default())
        .add_plugin(ParticlesPlugin)
        .add_plugin(
            WindowConfigPlugin::default()
                .maximized(true)
                .max_fps(120)
                .vsync(true),
        )
        .add_resource(State::default())
        .add_systems(OnSetup, setup_system)
        .add_systems(OnUpdate, update_system.before(ParticlesSysSet))
        .add_systems(OnRender, draw_system)
        .run()
}

fn setup_system(mut cmds: Commands, mut configs: ResMut<Particles>, window: Res<Window>) {
    configs.insert("my_fx".to_string(), ParticleFxConfig::default());
    cmds.spawn(
        configs
            .create_component("my_fx", window.size() * 0.5)
            .unwrap(),
    );
}

fn update_system(
    mut fx: Single<&mut ParticleFx>,
    mouse: Res<Mouse>,
    ctx: Res<EguiContext>,
    mut configs: ResMut<Particles>,
    mut state: ResMut<State>,
) {
    fx.spawning = true;

    if ctx.is_using_pointer() {
        return;
    }

    if mouse.is_down(MouseButton::Left) {
        if state.offset_edit_mode {
            if let Some(i) = state.selected_emitter {
                let cfg = configs.get_mut(&fx.id).unwrap();
                cfg.emitters[i].offset = mouse.position() - fx.pos;
                return;
            }
        }

        fx.pos = mouse.position();
    }
}

fn draw_system(
    fx: Single<&mut ParticleFx>,
    mut ectx: ResMut<EguiContext>,
    mut configs: ResMut<Particles>,
    mut state: ResMut<State>,
    time: Res<Time>,
) {
    let mut fx = fx.into_inner();
    let fx_id = fx.id.clone();
    let Some(cfg) = configs.get_mut(&fx_id) else {
        return;
    };

    // clear the backgroung
    let mut draw = create_draw_2d();
    draw.clear(state.clear_color);

    // draw the particle first
    draw.particle(&fx);

    if let Some(i) = state.selected_emitter {
        if state.offset_edit_mode {
            draw.circle(5.0)
                .color(Color::RED)
                .origin(Vec2::splat(0.5))
                .translate(fx.pos);

            let emitter_pos = fx.pos + cfg.emitters[i].offset;
            draw.line(fx.pos, emitter_pos)
                .color(Color::GREEN)
                .alpha(0.9)
                .width(2.0);

            draw.circle(2.0)
                .color(Color::RED)
                .origin(Vec2::splat(0.5))
                .translate(emitter_pos);

            match cfg.emitters[i].kind {
                EmitterKind::Point => {}
                EmitterKind::Rect(size) => {
                    draw.rect(Vec2::ZERO, size)
                        .origin(Vec2::splat(0.5))
                        .translate(emitter_pos)
                        .fill_color(Color::rgba(0.1, 0.3, 0.7, 0.5))
                        .fill()
                        .stroke_color(Color::WHITE)
                        .stroke(2.0);
                }
                EmitterKind::Circle(radius) => {
                    draw.circle(radius)
                        .origin(Vec2::splat(0.5))
                        .translate(emitter_pos)
                        .fill_color(Color::rgba(0.1, 0.3, 0.7, 0.5))
                        .fill()
                        .stroke_color(Color::WHITE)
                        .stroke(2.0);
                }
                EmitterKind::Ring { radius, width } => {
                    draw.circle(radius)
                        .origin(Vec2::splat(0.5))
                        .translate(emitter_pos)
                        .stroke_color(Color::rgba(0.1, 0.3, 0.7, 0.5))
                        .stroke(width);
                }
            }
        }
    }

    gfx::render_to_frame(&draw).unwrap();

    let fps = time.fps();
    let ms = time.delta_f32();
    let particles_amount = fx
        .emitters
        .iter()
        .fold(0, |acc, emitter| acc + emitter.particles.len());

    // draw the ui
    let edraw = ectx.run(|ctx| {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Load").clicked() {
                    // if let Some(path) = FileDialog::new()
                    //     .add_filter("Particle FX", &["json"])
                    //     .pick_file()
                    // {
                    //     match std::fs::read_to_string(&path).and_then(|s| {
                    //         serde_json::from_str::<ParticleFxConfig>(&s).map_err(Into::into)
                    //     }) {
                    //         Ok(cfg) => {
                    //             configs.insert("my_fx".to_string(), cfg.clone());
                    //             fx.reload_from_config(cfg);
                    //             println!("Loaded config from {:?}", path);
                    //         }
                    //         Err(e) => {
                    //             eprintln!("Load error: {}", e);
                    //         }
                    //     }
                    // }
                }

                if ui.button("Save").clicked() {
                    // if let Some(path) = FileDialog::new()
                    //     .set_file_name("particles.json")
                    //     .save_file()
                    // {
                    //     if let Some(cfg) = configs.get("my_fx") {
                    //         match serde_json::to_string_pretty(cfg)
                    //             .and_then(|j| fs::write(&path, j).map_err(Into::into))
                    //         {
                    //             Ok(_) => println!("Saved config to {:?}", path),
                    //             Err(e) => eprintln!("Save error: {}", e),
                    //         }
                    //     }
                    // }
                }

                ui.separator();

                // background color picker
                let mut rgb = state.clear_color.as_linear().to_rgb();
                ui.color_edit_button_rgb(&mut rgb);
                state.clear_color = Color::from_linear_rgb(LinearColor {
                    r: rgb[0],
                    g: rgb[1],
                    b: rgb[2],
                    a: 1.0,
                });

                ui.separator();

                ui.add_space(10.0);

                if ui.button("Restart").clicked() {
                    //
                }
            });
        });
        egui::SidePanel::left("left_panel")
            .min_width(300.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Emitters");

                if ui.small_button("Add Emitter").clicked() {
                    cfg.emitters.push(EmitterConfig::default());
                    fx.emitters.push(ParticleEmitter::default());

                    // select new emitter
                    let new_idx = cfg.emitters.len() - 1;
                    state.selected_emitter = Some(new_idx);
                }

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for i in 0..cfg.emitters.len() {
                        ui.horizontal(|ui| {
                            let is_selected = state.selected_emitter == Some(i);

                            let btn = ui.selectable_label(
                                is_selected,
                                format!("#{i}: {}", cfg.emitters[i].id),
                            );
                            if btn.clicked() {
                                state.selected_emitter = Some(i);
                            }

                            if ui.small_button("Down").clicked() && i + 1 < fx.emitters.len() {
                                let n = i + 1;
                                cfg.emitters.swap(i, n);
                                if is_selected {
                                    state.selected_emitter = Some(n);
                                }
                            }

                            if ui.small_button("Up").clicked() && i > 0 {
                                let n = i - 1;
                                cfg.emitters.swap(i, n);
                                if is_selected {
                                    state.selected_emitter = Some(n);
                                }
                            }
                        });
                    }
                });

                let mut remove_idx: Option<usize> = None;
                if let Some(i) = state.selected_emitter {
                    if i < cfg.emitters.len() {
                        ui.separator();
                        ui.group(|ui| {
                            ui.heading(format!("Emitter #{i} Properties:"));

                            ui.horizontal(|ui| {
                                ui.label("Id:");
                                ui.text_edit_singleline(&mut cfg.emitters[i].id);
                            });

                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.small_button("Reset").clicked() {
                                    cfg.emitters[i] = Default::default();
                                    fx.emitters[i] = Default::default();
                                }

                                if ui.small_button("Remove").clicked() {
                                    remove_idx = Some(i);
                                    if cfg.emitters.is_empty() {
                                        state.selected_emitter = None;
                                    } else {
                                        state.selected_emitter = Some(0);
                                    }
                                }
                            });
                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label("Offset X: ");
                                ui.add(
                                    egui::DragValue::new(&mut cfg.emitters[i].offset.x).speed(1.0),
                                );
                                ui.separator();
                                ui.label("Offset Y: ");
                                ui.add(
                                    egui::DragValue::new(&mut cfg.emitters[i].offset.y).speed(1.0),
                                );
                                ui.separator();
                                ui.toggle_value(&mut state.offset_edit_mode, "Visual Mode");
                            });

                            ui.horizontal(|ui| {
                                ui.label("Shape: ");
                                egui::ComboBox::from_label("")
                                    .selected_text(cfg.emitters[i].kind.as_ref())
                                    .show_ui(ui, |ui| {
                                        let point = EmitterKind::Point;
                                        ui.selectable_value(
                                            &mut cfg.emitters[i].kind,
                                            point,
                                            point.as_ref(),
                                        );

                                        let square = EmitterKind::Rect(Vec2::splat(150.0));
                                        ui.selectable_value(
                                            &mut cfg.emitters[i].kind,
                                            square,
                                            square.as_ref(),
                                        );

                                        let circle = EmitterKind::Circle(100.0);
                                        ui.selectable_value(
                                            &mut cfg.emitters[i].kind,
                                            circle,
                                            circle.as_ref(),
                                        );

                                        let ring = EmitterKind::Ring {
                                            radius: 100.0,
                                            width: 20.0,
                                        };
                                        ui.selectable_value(
                                            &mut cfg.emitters[i].kind,
                                            ring,
                                            ring.as_ref(),
                                        );
                                    });
                            });

                            match &mut cfg.emitters[i].kind {
                                EmitterKind::Rect(size) => {
                                    ui.horizontal(|ui| {
                                        ui.label("Width: ");
                                        ui.add(
                                            egui::DragValue::new(&mut size.x)
                                                .range(1.0..=1000.0)
                                                .clamp_existing_to_range(true)
                                                .speed(1.0),
                                        );

                                        ui.separator();

                                        ui.label("Height: ");
                                        ui.add(
                                            egui::DragValue::new(&mut size.y)
                                                .range(1.0..=1000.0)
                                                .clamp_existing_to_range(true)
                                                .speed(1.0),
                                        );
                                    });
                                }
                                EmitterKind::Circle(radius) => {
                                    ui.horizontal(|ui| {
                                        ui.label("Radius: ");
                                        ui.add(
                                            egui::DragValue::new(radius)
                                                .range(1.0..=1000.0)
                                                .clamp_existing_to_range(true)
                                                .speed(1.0),
                                        );
                                    });
                                }
                                EmitterKind::Ring { radius, width } => {
                                    ui.horizontal(|ui| {
                                        ui.label("Radius: ");
                                        ui.add(
                                            egui::DragValue::new(radius)
                                                .range(1.0..=1000.0)
                                                .clamp_existing_to_range(true)
                                                .speed(1.0),
                                        );

                                        ui.label("Width: ");
                                        ui.add(
                                            egui::DragValue::new(width)
                                                .range(1.0..=1000.0)
                                                .clamp_existing_to_range(true)
                                                .speed(1.0),
                                        );
                                    });
                                }
                                EmitterKind::Point => {}
                            }

                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label("Particles per wave: ");
                                ui.add(egui::Slider::new(
                                    &mut cfg.emitters[i].particles_per_wave,
                                    1..=30_000,
                                ));
                            });

                            ui.horizontal(|ui| {
                                ui.label("Wave time: ");
                                ui.add(egui::Slider::new(
                                    &mut cfg.emitters[i].wave_time,
                                    0.017..=60.0,
                                ));
                            });

                            ui.horizontal(|ui| {
                                ui.label("Delay: ");
                                ui.add(egui::Slider::new(&mut cfg.emitters[i].delay, 0.0..=60.0));
                            });

                            ui.horizontal(|ui| {
                                ui.label("Repeats: ");
                                let mut is_some = cfg.emitters[i].repeat.is_some();
                                let txt = if is_some { "Yes" } else { "No" };
                                ui.toggle_value(&mut is_some, txt);

                                if is_some {
                                    let mut n = cfg.emitters[i].repeat.unwrap_or(1);
                                    ui.add(
                                        egui::DragValue::new(&mut n)
                                            .speed(0.1)
                                            .range(1..=100)
                                            .clamp_existing_to_range(true),
                                    );

                                    cfg.emitters[i].repeat = Some(n);
                                } else {
                                    cfg.emitters[i].repeat = None;
                                }
                            });
                        });
                    }
                }

                if let Some(idx) = remove_idx {
                    cfg.emitters.remove(idx);
                    fx.emitters.remove(idx);
                }
            });

        egui::SidePanel::right("right_panel")
            .resizable(false)
            .min_width(300.0)
            .show(ctx, |ui| {
                if let Some(i) = state.selected_emitter {
                    if i < cfg.emitters.len() {
                        ui.separator();
                        ui.group(|ui| {
                            ui.heading(format!("Emitter #{i} Attributes:"));
                            ui.separator();

                            egui::CollapsingHeader::new("Lifetime: ")
                                .default_open(true)
                                .show(ui, |ui| {
                                    value_box(
                                        ui,
                                        &mut cfg.emitters[i].attributes.lifetime,
                                        "lifetime",
                                    );
                                });

                            ui.separator();

                            egui::CollapsingHeader::new("Scale").show(ui, |ui| {
                                ui.label("Initial X: ");
                                value_box(
                                    ui,
                                    &mut cfg.emitters[i].attributes.scale_x.initial,
                                    "init_scale_x",
                                );

                                ui.label("Initial Y: ");
                                value_box(
                                    ui,
                                    &mut cfg.emitters[i].attributes.scale_y.initial,
                                    "init_scale_y",
                                );
                                ui.separator();
                                ui.label("Behavior X: ");
                                behavior_box(
                                    ui,
                                    &mut cfg.emitters[i].attributes.scale_x.behavior,
                                    2.0,
                                    "behavior_scale_x",
                                );
                                ui.label("Behavior Y: ");
                                behavior_box(
                                    ui,
                                    &mut cfg.emitters[i].attributes.scale_y.behavior,
                                    2.0,
                                    "behavior_scale_y",
                                );
                            });

                            egui::CollapsingHeader::new("Speed").show(ui, |ui| {
                                ui.label("Initial: ");
                                value_box(
                                    ui,
                                    &mut cfg.emitters[i].attributes.speed.initial,
                                    "init_speed",
                                );

                                ui.separator();
                                ui.label("Behavior: ");
                                behavior_box(
                                    ui,
                                    &mut cfg.emitters[i].attributes.speed.behavior,
                                    150.0,
                                    "behavior_speed",
                                );
                            });
                        });
                    }
                }
            });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Fps: {fps:.0}"));
                ui.separator();
                ui.label(format!("Delta: {ms:.2} ms"));
                ui.separator();
                ui.label(format!("Particles: {particles_amount}"));
            });
        });
    });

    gfx::render_to_frame(&edraw).unwrap();
}

fn value_box(ui: &mut egui::Ui, value: &mut Value<f32>, id: &str) {
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(value.as_ref())
            .show_ui(ui, |ui| {
                let val = Value::Fixed(2.0);
                ui.selectable_value(value, val, val.as_ref());

                let val = Value::Range { min: 0.5, max: 3.0 };
                ui.selectable_value(value, val, val.as_ref());
            });

        match value {
            Value::Fixed(val) => {
                ui.add(egui::Slider::new(val, 0.1..=100.0));
            }
            Value::Range { min, max } => {
                ui.label("Min: ");
                ui.add(
                    egui::DragValue::new(min)
                        .range(0.1..=1000.0)
                        .clamp_existing_to_range(true)
                        .speed(0.1),
                );

                ui.separator();

                ui.label("Max: ");
                ui.add(
                    egui::DragValue::new(max)
                        .range(*min..=1000.0)
                        .clamp_existing_to_range(true)
                        .speed(0.1),
                );
            }
        }
    });
}

fn behavior_box(ui: &mut egui::Ui, value: &mut Option<Behavior<f32>>, to: f32, id: &str) {
    ui.horizontal(|ui| {
        let selected = value
            .as_ref()
            .map_or("None", |v| {
                let s: &str = v.as_ref();
                s
            })
            .to_string();

        egui::ComboBox::from_id_salt(id)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.selectable_value(value, None, "None");

                let val = Behavior::To {
                    value: to,
                    curve: Curve::Linear,
                };
                ui.selectable_value(value, Some(val.clone()), val.as_ref());

                let val = Behavior::Increment(to * 0.017);
                ui.selectable_value(value, Some(val.clone()), val.as_ref());
            });
    });

    ui.horizontal(|ui| match value {
        Some(Behavior::To { value, curve }) => {
            ui.label("To: ");
            ui.add(egui::DragValue::new(value).speed(0.1));

            // TODO: curve
        }
        Some(Behavior::Increment(inc)) => {
            ui.label("Amount: ");
            ui.add(egui::DragValue::new(inc).speed(0.1));
        }
        None => {}
    });
}
