use std::{
    f32::consts::TAU,
    task::{Context, Poll},
};

use draw::{Sprite, Transform2D, create_sprite};
use egui::{Align, Color32, load::SizedTexture};
use futures::{future::BoxFuture, task::noop_waker_ref};
use rfd::{AsyncFileDialog, FileHandle};
use rkit::{
    draw::create_draw_2d,
    prelude::*,
    egui::{EguiContext, EguiPlugin, Layout},
    gfx::{self, Color, LinearColor},
    math::{Vec2, Vec3, vec3},
    particles::*,
};
use rustc_hash::FxHashMap;
use strum::IntoEnumIterator;

const EXT: &str = "gkp";

enum FileCmd {
    Save(BoxFuture<'static, Option<FileHandle>>),
    Load(BoxFuture<'static, Option<FileHandle>>),
    LoadTexture(BoxFuture<'static, Option<FileHandle>>),
}

struct FileCmdRes(Option<FileCmd>);

struct EguiSprite {
    sized: SizedTexture,
    sprite: Sprite,
}

#[derive(Resource)]
struct State {
    clear_color: Color,
    selected_emitter: Option<usize>,
    offset_edit_mode: bool,
    zoom: f32,
    file_name: String,
    sprites: FxHashMap<String, EguiSprite>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            clear_color: Color::BLACK,
            selected_emitter: Some(0),
            offset_edit_mode: false,
            zoom: 1.0,
            file_name: format!("my_particle.{EXT}"),
            sprites: FxHashMap::default(),
        }
    }
}

fn main() -> Result<(), String> {
    App::new()
        .insert_non_send_resource(FileCmdRes(None))
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
        .on_setup(setup_system.after(ParticlesSysSet))
        .on_update(update_system.before(ParticlesSysSet))
        .on_render(draw_system)
        .run()
}

fn setup_system(
    mut cmds: Commands,
    config: Res<Particles>,
    mut state: ResMut<State>,
    mut ectx: ResMut<EguiContext>,
) {
    cmds.queue(LoadParticleConfigCmd {
        config: ParticleFxConfig::default(),
    });

    config.iter_sprites().for_each(|(id, sprite)| {
        let sized_tex = ectx.add_sprite(sprite);
        state.sprites.insert(
            id.clone(),
            EguiSprite {
                sized: sized_tex,
                sprite: sprite.clone(),
            },
        );
    });
}

fn update_system(
    mut cmds: Commands,
    mut fx: Single<&mut ParticleFx>,
    mouse: Res<Mouse>,
    mut ectx: ResMut<EguiContext>,
    window: Res<Window>,
    mut state: ResMut<State>,
    mut particles: ResMut<Particles>,
    mut file_cmd: NonSendMut<FileCmdRes>,
) {
    //set emitter definitions
    if let Some(cfg) = particles.get_config("my_fx") {
        cfg.emitters.iter().enumerate().for_each(|(n, cfg)| {
            fx.emitters[n].def = cfg.def.clone();
        });
    }

    if ectx.is_using_pointer() {
        return;
    }

    if mouse.is_down(MouseButton::Left) {
        let center = window.size() * 0.5;
        let mouse_world = (mouse.position() - center) / state.zoom + center;

        if state.offset_edit_mode {
            if let Some(i) = state.selected_emitter {
                let cfg = particles.get_config_mut(&fx.id).unwrap();
                cfg.emitters[i].def.offset = mouse_world - fx.pos;
                return;
            }
        }
        fx.pos = mouse_world;
    }

    if let Some(cmd) = &mut file_cmd.0 {
        match cmd {
            FileCmd::Load(fut) => {
                let waker = noop_waker_ref();
                let mut cx = Context::from_waker(waker);

                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(opt_handle) => {
                        if let Some(handle) = opt_handle {
                            match std::fs::read_to_string(handle.path()).and_then(|s| {
                                serde_json::from_str::<ParticleFxConfig>(&s).map_err(Into::into)
                            }) {
                                Ok(cfg) => {
                                    cmds.queue(LoadParticleConfigCmd { config: cfg });
                                    state.file_name = handle.file_name();
                                }
                                Err(e) => {
                                    eprintln!("Load error: {e}");
                                }
                            }
                        }
                        file_cmd.0 = None;
                    }
                    Poll::Pending => {}
                }
            }
            FileCmd::Save(fut) => {
                let waker = noop_waker_ref();
                let mut cx = Context::from_waker(waker);

                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(opt_handle) => {
                        if let Some(handle) = opt_handle {
                            let cfg = particles.get_config("my_fx").unwrap();
                            match serde_json::to_string_pretty(cfg)
                                .map_err(|e| e.to_string())
                                .and_then(|j| {
                                    println!("Saving {j}");
                                    std::fs::write(handle.path(), j).map_err(|e| e.to_string())
                                }) {
                                Ok(_) => println!("Saved config to {:?}", handle.path()),
                                Err(e) => eprintln!("Save error: {e}"),
                            }
                        }
                        file_cmd.0 = None;
                    }
                    Poll::Pending => {}
                }
            }
            FileCmd::LoadTexture(fut) => {
                let waker = noop_waker_ref();
                let mut cx = Context::from_waker(waker);
                if let Poll::Ready(opt) = fut.as_mut().poll(&mut cx) {
                    if let Some(handle) = opt {
                        if let Ok(bytes) = std::fs::read(handle.path()) {
                            if let Ok(sprite) = create_sprite().from_image(&bytes).build() {
                                let id = handle.file_name();
                                state.sprites.insert(
                                    id.clone(),
                                    EguiSprite {
                                        sized: ectx.add_sprite(&sprite),
                                        sprite: sprite.clone(),
                                    },
                                );

                                //  sync particle
                                if let Some(cfg) = particles.get_config_mut(&fx.id) {
                                    let emitter_idx = state.selected_emitter.unwrap_or(0);
                                    if !cfg.emitters[emitter_idx].sprites.iter().any(|ps| matches!(ps, ParticleSprite::Id(existing) if existing == &id)) {
                                        cfg.emitters[emitter_idx].sprites.push(ParticleSprite::Id(id.clone()));
                                    }

                                    fx.emitters[emitter_idx].sprites = cfg.emitters[emitter_idx]
                                        .sprites
                                        .iter()
                                        .filter_map(|ps| match ps {
                                            ParticleSprite::Id(id) => {
                                                state.sprites.get(id).map(|es| es.sprite.clone())
                                            }
                                            ParticleSprite::Sprite { sprite, .. } => {
                                                Some(sprite.clone())
                                            }
                                        })
                                        .collect();
                                }
                            }
                        }
                    }
                    file_cmd.0 = None;
                }
            }
        }
    }
}

fn draw_system(
    mut cmds: Commands,
    fx: Single<&mut ParticleFx>,
    time: Res<Time>,
    window: Res<Window>,
    mut ectx: ResMut<EguiContext>,
    mut particles: ResMut<Particles>,
    mut state: ResMut<State>,
    mut file_cmd: NonSendMut<FileCmdRes>,
) {
    let mut fx = fx.into_inner();
    let fx_id = fx.id.clone();
    let Some(cfg) = particles.get_config_mut(&fx_id) else {
        return;
    };

    // clear the backgroung
    let mut draw = create_draw_2d();
    draw.clear(state.clear_color);

    draw.push_matrix(
        Transform2D::builder()
            .set_size(window.size())
            .set_translation(window.size() * 0.5)
            .set_origin(Vec2::splat(0.5))
            .set_scale(Vec2::splat(state.zoom))
            .build()
            .as_mat3(),
    );

    // draw the particle first
    draw.particle(&fx);

    if let Some(i) = state.selected_emitter {
        if state.offset_edit_mode {
            draw.circle(5.0)
                .color(Color::RED)
                .origin(Vec2::splat(0.5))
                .translate(fx.pos);

            let emitter_pos = fx.pos + cfg.emitters[i].def.offset;
            draw.line(fx.pos, emitter_pos)
                .color(Color::GREEN)
                .alpha(0.9)
                .width(2.0);

            let emitter_rot = cfg.emitters[i].def.rotation;
            draw.push_matrix(
                Transform2D::builder()
                    .set_origin(Vec2::splat(0.5))
                    .set_translation(emitter_pos)
                    .set_rotation(emitter_rot)
                    .build()
                    .as_mat3(),
            );
            draw.circle(2.0)
                .color(Color::RED)
                .origin(Vec2::splat(0.5))
                .translate(emitter_pos);

            match cfg.emitters[i].def.kind {
                EmitterShape::Point => {}
                EmitterShape::Rect(size) => {
                    draw.rect(Vec2::ZERO, size)
                        .origin(Vec2::splat(0.5))
                        .fill_color(Color::rgba(0.1, 0.3, 0.7, 0.5))
                        .fill()
                        .stroke_color(Color::WHITE)
                        .stroke(2.0);
                }
                EmitterShape::Circle(radius) => {
                    draw.circle(radius)
                        .origin(Vec2::splat(0.5))
                        .fill_color(Color::rgba(0.1, 0.3, 0.7, 0.5))
                        .fill()
                        .stroke_color(Color::WHITE)
                        .stroke(2.0);
                }
                EmitterShape::Ring { radius, width } => {
                    draw.circle(radius)
                        .origin(Vec2::splat(0.5))
                        .stroke_color(Color::rgba(0.1, 0.3, 0.7, 0.5))
                        .stroke(width);
                }
                EmitterShape::Radial { count, radius } => {
                    for i in 0..count {
                        let angle = TAU * (i as f32) / (count as f32);
                        let local_pos = Vec2::from_angle(angle) * radius;

                        draw.circle(5.0)
                            .origin(Vec2::splat(0.5))
                            .translate(local_pos)
                            .fill_color(Color::rgba(0.1, 0.3, 0.7, 0.5))
                            .fill()
                            .stroke_color(Color::WHITE)
                            .stroke(2.0);
                    }
                }
            }
            draw.pop_matrix();
        }
    }

    draw.pop_matrix();

    gfx::render_to_frame(&draw).unwrap();

    let fps = time.fps();
    let ms = time.delta_f32() * 1000.0;
    let particles_amount = fx
        .emitters
        .iter()
        .fold(0, |acc, emitter| acc + emitter.particles.len());

    // draw the ui
    let edraw = ectx.run(|ctx| {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Load").clicked() {
                    file_cmd.0 = Some(FileCmd::Load(Box::pin(async {
                        AsyncFileDialog::new()
                            .add_filter("particles", &[EXT])
                            .pick_file()
                            .await
                    })));
                }

                if ui.button("Save").clicked() {
                    let file_name = state.file_name.clone();
                    file_cmd.0 = Some(FileCmd::Save(Box::pin(async move {
                        AsyncFileDialog::new()
                            .set_file_name(&file_name)
                            .save_file()
                            .await
                    })));
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
                ui.horizontal(|ui| {
                    let btn_state = if fx.spawning { "Stop" } else { "Start" };
                    if ui.button(btn_state).clicked() {
                        fx.spawning = !fx.spawning;
                        fx.reset();
                    }

                    ui.checkbox(&mut fx.is_local, "Is Local");
                });
                ui.separator();

                ui.heading("Emitters");

                if ui.small_button("Add Emitter").clicked() {
                    cfg.emitters.push(EmitterConfig::default());
                    fx.emitters.push(ParticleEmitter {
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
                            sort: SortBy::SpawnOnTop,
                            attributes: Attributes::default(),
                        },
                        spawn_accumulator: 0.0,
                        time: 0.0,
                        delay: 0.0,
                        ended: false,
                        repeats: 0,
                        sprites: vec![],
                        particles: vec![],
                    });

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
                                    egui::DragValue::new(&mut cfg.emitters[i].def.offset.x)
                                        .speed(1.0),
                                );
                                ui.separator();
                                ui.label("Offset Y: ");
                                ui.add(
                                    egui::DragValue::new(&mut cfg.emitters[i].def.offset.y)
                                        .speed(1.0),
                                );
                                ui.separator();
                                ui.toggle_value(&mut state.offset_edit_mode, "Visual Mode");
                            });

                            ui.horizontal(|ui| {
                                ui.label("Rotation: ");
                                let mut rot = cfg.emitters[i].def.rotation.to_degrees();
                                ui.add(egui::Slider::new(&mut rot, -360.0f32..=360.0));
                                cfg.emitters[i].def.rotation = rot.to_radians();
                            });

                            ui.horizontal(|ui| {
                                ui.label("Sort: ");
                                let value = &mut cfg.emitters[i].def.sort;
                                egui::ComboBox::from_id_salt("spawn_sort")
                                    .selected_text(value.as_ref())
                                    .show_ui(ui, |ui| {
                                        SortBy::iter().for_each(|val| {
                                            ui.selectable_value(value, val, val.as_ref());
                                        });
                                    });
                            });

                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label("Shape: ");
                                egui::ComboBox::from_label("")
                                    .selected_text(cfg.emitters[i].def.kind.as_ref())
                                    .show_ui(ui, |ui| {
                                        let point = EmitterShape::Point;
                                        ui.selectable_value(
                                            &mut cfg.emitters[i].def.kind,
                                            point,
                                            point.as_ref(),
                                        );

                                        let square = EmitterShape::Rect(Vec2::splat(150.0));
                                        ui.selectable_value(
                                            &mut cfg.emitters[i].def.kind,
                                            square,
                                            square.as_ref(),
                                        );

                                        let circle = EmitterShape::Circle(100.0);
                                        ui.selectable_value(
                                            &mut cfg.emitters[i].def.kind,
                                            circle,
                                            circle.as_ref(),
                                        );

                                        let ring = EmitterShape::Ring {
                                            radius: 100.0,
                                            width: 20.0,
                                        };
                                        ui.selectable_value(
                                            &mut cfg.emitters[i].def.kind,
                                            ring,
                                            ring.as_ref(),
                                        );

                                        let burst = EmitterShape::Radial {
                                            count: 8,
                                            radius: 100.0,
                                        };
                                        ui.selectable_value(
                                            &mut cfg.emitters[i].def.kind,
                                            burst,
                                            burst.as_ref(),
                                        );
                                    });
                            });

                            match &mut cfg.emitters[i].def.kind {
                                EmitterShape::Rect(size) => {
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
                                EmitterShape::Circle(radius) => {
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
                                EmitterShape::Ring { radius, width } => {
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
                                EmitterShape::Radial { count, radius } => {
                                    ui.horizontal(|ui| {
                                        ui.label("Radius: ");
                                        ui.add(
                                            egui::DragValue::new(radius)
                                                .range(1.0..=1000.0)
                                                .clamp_existing_to_range(true)
                                                .speed(1.0),
                                        );

                                        ui.label("Count: ");
                                        ui.add(
                                            egui::DragValue::new(count)
                                                .range(1..=50)
                                                .clamp_existing_to_range(true)
                                                .speed(1.0),
                                        );
                                    });
                                }
                                EmitterShape::Point => {}
                            }

                            ui.separator();

                            ui.horizontal(|ui| {
                                //
                                ui.label("Gravity: ");
                                ui.add(
                                    egui::DragValue::new(&mut cfg.emitters[i].def.gravity.amount)
                                        .speed(1.0),
                                );

                                ui.separator();
                                ui.label("Angle: ");
                                let mut rot = cfg.emitters[i].def.gravity.angle.to_degrees();
                                ui.add(egui::Slider::new(&mut rot, -360.0..=360.0));
                                cfg.emitters[i].def.gravity.angle = rot.to_radians();
                            });

                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label("Particles per wave: ");
                                ui.add(egui::Slider::new(
                                    &mut cfg.emitters[i].def.particles_per_wave,
                                    1..=30_000,
                                ));
                            });

                            ui.horizontal(|ui| {
                                ui.label("Wave time: ");
                                ui.add(egui::Slider::new(
                                    &mut cfg.emitters[i].def.wave_time,
                                    0.017..=60.0,
                                ));
                            });

                            ui.horizontal(|ui| {
                                ui.label("Delay: ");
                                ui.add(egui::Slider::new(
                                    &mut cfg.emitters[i].def.delay,
                                    0.0..=60.0,
                                ));
                            });

                            ui.horizontal(|ui| {
                                ui.label("Repeats: ");
                                let mut is_some = cfg.emitters[i].def.repeat.is_some();
                                let txt = if is_some { "Yes" } else { "No" };
                                ui.toggle_value(&mut is_some, txt);

                                if is_some {
                                    let mut n = cfg.emitters[i].def.repeat.unwrap_or(1);
                                    ui.add(
                                        egui::DragValue::new(&mut n)
                                            .speed(0.1)
                                            .range(1..=100)
                                            .clamp_existing_to_range(true),
                                    );

                                    cfg.emitters[i].def.repeat = Some(n);
                                } else {
                                    cfg.emitters[i].def.repeat = None;
                                }
                            });

                            ui.separator();

                            ui.heading("Textures:");

                            ui.horizontal(|ui| {
                                if ui.small_button("➕ Load…").clicked() {
                                    file_cmd.0 = Some(FileCmd::LoadTexture(Box::pin(async {
                                        AsyncFileDialog::new()
                                            .add_filter("image", &["png", "jpg", "jpeg"])
                                            .pick_file()
                                            .await
                                    })));
                                }
                            });

                            ui.separator();

                            egui::ScrollArea::vertical()
                                .auto_shrink(false)
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        for (ps_id, es) in &state.sprites {
                                            let selected =
                                                cfg.emitters[i].sprites.iter().any(|ps| match ps {
                                                    ParticleSprite::Id(existing) => {
                                                        existing == ps_id
                                                    }
                                                    ParticleSprite::Sprite { sprite, .. } => {
                                                        sprite == &es.sprite
                                                    }
                                                });
                                            // .any(|ps| matches!(ps, ParticleSprite::Id(existing) if existing == id));

                                            let frame = es.sprite.frame();
                                            let min = frame.min();
                                            let max = frame.max();
                                            let uv_min = min / es.sprite.texture().size();
                                            let uv_max = max / es.sprite.texture().size();

                                            let img = egui::Image::new(es.sized)
                                                .uv(egui::Rect {
                                                    min: egui::Pos2 {
                                                        x: uv_min.x,
                                                        y: uv_min.y,
                                                    },
                                                    max: egui::Pos2 {
                                                        x: uv_max.x,
                                                        y: uv_max.y,
                                                    },
                                                })
                                                .fit_to_exact_size(egui::Vec2::new(32.0, 32.0))
                                                .bg_fill(if selected {
                                                    Color32::from_rgb(50, 50, 100)
                                                } else {
                                                    Color32::BLACK
                                                });

                                            let thumb = egui::ImageButton::new(img)
                                                .frame(false)
                                                .selected(selected);
                                            // .on_hover_text(id);

                                            if ui.add(thumb).clicked() {
                                                if selected {
                                                    cfg.emitters[i].sprites.retain(|ps| match ps {
                                                        ParticleSprite::Id(existing) => {
                                                            existing != ps_id
                                                        }
                                                        ParticleSprite::Sprite {
                                                            sprite, ..
                                                        } => sprite != &es.sprite,
                                                    });
                                                } else {
                                                    cfg.emitters[i]
                                                        .sprites
                                                        .push(ParticleSprite::Id(ps_id.clone()));
                                                }
                                                fx.emitters[i].sprites = cfg.emitters[i]
                                                    .sprites
                                                    .iter()
                                                    .filter_map(|ps| match ps {
                                                        ParticleSprite::Id(id) => state
                                                            .sprites
                                                            .get(id)
                                                            .map(|es| es.sprite.clone()),
                                                        ParticleSprite::Sprite {
                                                            sprite, ..
                                                        } => Some(sprite.clone()),
                                                    })
                                                    .collect();
                                            }
                                        }
                                    });
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
                                        &mut cfg.emitters[i].def.attributes.lifetime,
                                        "lifetime",
                                    );
                                });

                            ui.separator();

                            egui::CollapsingHeader::new("Scale").show(ui, |ui| {
                                ui.label("Initial X: ");
                                value_box(
                                    ui,
                                    &mut cfg.emitters[i].def.attributes.scale_x.initial,
                                    "init_scale_x",
                                );

                                ui.label("Initial Y: ");
                                value_box(
                                    ui,
                                    &mut cfg.emitters[i].def.attributes.scale_y.initial,
                                    "init_scale_y",
                                );
                                ui.separator();
                                ui.label("Behavior X: ");
                                behavior_box(
                                    ui,
                                    &mut cfg.emitters[i].def.attributes.scale_x.behavior,
                                    2.0,
                                    "behavior_scale_x",
                                );
                                ui.label("Behavior Y: ");
                                behavior_box(
                                    ui,
                                    &mut cfg.emitters[i].def.attributes.scale_y.behavior,
                                    2.0,
                                    "behavior_scale_y",
                                );
                            });

                            ui.separator();
                            egui::CollapsingHeader::new("Speed").show(ui, |ui| {
                                ui.label("Initial: ");
                                value_box(
                                    ui,
                                    &mut cfg.emitters[i].def.attributes.speed.initial,
                                    "init_speed",
                                );

                                ui.separator();
                                ui.label("Behavior: ");
                                behavior_box(
                                    ui,
                                    &mut cfg.emitters[i].def.attributes.speed.behavior,
                                    150.0,
                                    "behavior_speed",
                                );
                            });

                            ui.separator();
                            egui::CollapsingHeader::new("Rotation").show(ui, |ui| {
                                ui.label("Initial: ");
                                value_box_angle(
                                    ui,
                                    &mut cfg.emitters[i].def.attributes.rotation.attr.initial,
                                    "init_rotation",
                                );

                                ui.separator();
                                ui.label("Behavior: ");

                                #[derive(Debug, Clone, PartialEq)]
                                enum RotVal {
                                    Lock,
                                    Behave(Behavior<f32>),
                                }

                                let mut value =
                                    if cfg.emitters[i].def.attributes.rotation.lock_to_angle {
                                        Some(RotVal::Lock)
                                    } else {
                                        cfg.emitters[i]
                                            .def
                                            .attributes
                                            .rotation
                                            .attr
                                            .behavior
                                            .clone()
                                            .map(RotVal::Behave)
                                    };

                                ui.horizontal(|ui| {
                                    let selected = value
                                        .as_ref()
                                        .map_or("None", |v| {
                                            let s: &str = match v {
                                                RotVal::Lock => "LockToAngle",
                                                RotVal::Behave(behavior) => behavior.as_ref(),
                                            };
                                            s
                                        })
                                        .to_string();

                                    egui::ComboBox::from_id_salt("rot_behavior")
                                        .selected_text(selected)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut value, None, "None");

                                            ui.selectable_value(
                                                &mut value,
                                                Some(RotVal::Lock),
                                                "LockToAngle",
                                            );

                                            let val = RotVal::Behave(Behavior::To {
                                                value: 180.0,
                                                curve: Curve::Linear,
                                            });
                                            ui.selectable_value(
                                                &mut value,
                                                Some(val.clone()),
                                                "To",
                                            );

                                            let val = RotVal::Behave(Behavior::Increment(1.0));
                                            ui.selectable_value(
                                                &mut value,
                                                Some(val.clone()),
                                                "Increment",
                                            );
                                        });

                                    match value {
                                        Some(RotVal::Lock) => {
                                            cfg.emitters[i].def.attributes.rotation.lock_to_angle =
                                                true;
                                            cfg.emitters[i].def.attributes.rotation.attr.behavior =
                                                None;
                                        }
                                        Some(RotVal::Behave(mut behavior)) => {
                                            match &mut behavior {
                                                Behavior::To { value, curve } => {
                                                    ui.label("To: ");
                                                    let mut rot = value.to_degrees();
                                                    ui.add(egui::Slider::new(
                                                        &mut rot,
                                                        -360f32..=360.0,
                                                    ));
                                                    *value = rot.to_radians();

                                                    ui.separator();
                                                    egui::ComboBox::from_id_salt(format!(
                                                        "{rot}_curve"
                                                    ))
                                                    .selected_text(curve.as_ref())
                                                    .show_ui(ui, |ui| {
                                                        Curve::iter().for_each(|c| {
                                                            ui.selectable_value(
                                                                curve,
                                                                c.clone(),
                                                                c.as_ref(),
                                                            );
                                                        });
                                                    });
                                                }
                                                Behavior::Increment(inc) => {
                                                    ui.label("Amount: ");
                                                    ui.add(egui::DragValue::new(inc).speed(0.1));
                                                }
                                            }

                                            cfg.emitters[i].def.attributes.rotation.lock_to_angle =
                                                false;
                                            cfg.emitters[i].def.attributes.rotation.attr.behavior =
                                                Some(behavior);
                                        }
                                        None => {
                                            cfg.emitters[i].def.attributes.rotation.lock_to_angle =
                                                false;
                                            cfg.emitters[i].def.attributes.rotation.attr.behavior =
                                                None;
                                        }
                                    }
                                });
                            });

                            ui.separator();
                            egui::CollapsingHeader::new("Direction").show(ui, |ui| {
                                ui.label("Initial: ");
                                value_box_angle(
                                    ui,
                                    &mut cfg.emitters[i].def.attributes.direction.initial,
                                    "init_direction",
                                );

                                ui.separator();
                                ui.label("Behavior: ");
                                behavior_box_angle(
                                    ui,
                                    &mut cfg.emitters[i].def.attributes.direction.behavior,
                                    1.0,
                                    "behavior_direction",
                                );
                            });

                            ui.separator();
                            egui::CollapsingHeader::new("Color").show(ui, |ui| {
                                ui.label("Initial: ");
                                ui.horizontal(|ui| {
                                    let init_value =
                                        &mut cfg.emitters[i].def.attributes.rgb.initial;
                                    egui::ComboBox::from_id_salt("init_color")
                                        .selected_text(init_value.as_ref())
                                        .show_ui(ui, |ui| {
                                            let val = Value::Fixed(Vec3::splat(1.0));
                                            ui.selectable_value(init_value, val, val.as_ref());

                                            let val = Value::Range {
                                                min: Vec3::splat(0.0),
                                                max: Vec3::splat(1.0),
                                            };
                                            ui.selectable_value(init_value, val, val.as_ref());
                                        });

                                    match init_value {
                                        Value::Fixed(val) => {
                                            let mut rgb = linear_rgb_from(*val);
                                            ui.color_edit_button_rgb(&mut rgb);
                                            *val = gamme_rgb_from(rgb);
                                        }
                                        Value::Range { min, max } => {
                                            ui.label("Min: ");
                                            let mut min_rgb = linear_rgb_from(*min);
                                            ui.color_edit_button_rgb(&mut min_rgb);
                                            *min = gamme_rgb_from(min_rgb);

                                            ui.separator();

                                            ui.label("Max: ");
                                            let mut max_rgb = linear_rgb_from(*max);
                                            ui.color_edit_button_rgb(&mut max_rgb);
                                            *max = gamme_rgb_from(max_rgb);
                                        }
                                    }
                                });
                                ui.separator();

                                ui.label("Behavior: ");
                                ui.horizontal(|ui| {
                                    let value = &mut cfg.emitters[i].def.attributes.rgb.behavior;
                                    let selected = value
                                        .as_ref()
                                        .map_or("None", |v| {
                                            let s: &str = v.as_ref();
                                            s
                                        })
                                        .to_string();

                                    egui::ComboBox::from_id_salt("behavior_rgb")
                                        .selected_text(selected)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(value, None, "None");

                                            let val = ColorBehavior::Simple(Behavior::To {
                                                value: Vec3::splat(1.0),
                                                curve: Curve::Linear,
                                            });
                                            ui.selectable_value(
                                                value,
                                                Some(val.clone()),
                                                val.as_ref(),
                                            );

                                            let val = ColorBehavior::ByChannel {
                                                red: None,
                                                green: None,
                                                blue: None,
                                            };
                                            ui.selectable_value(
                                                value,
                                                Some(val.clone()),
                                                val.as_ref(),
                                            );
                                        });

                                    if let Some(ColorBehavior::Simple(behavior)) = value {
                                        egui::ComboBox::from_id_salt("behavior_rgb_simple")
                                            .selected_text(behavior.as_ref())
                                            .show_ui(ui, |ui| {
                                                let val = Behavior::To {
                                                    value: Vec3::splat(1.0),
                                                    curve: Curve::Linear,
                                                };
                                                ui.selectable_value(
                                                    behavior,
                                                    val.clone(),
                                                    val.as_ref(),
                                                );

                                                let val = Behavior::Increment(Vec3::splat(0.0));
                                                ui.selectable_value(
                                                    behavior,
                                                    val.clone(),
                                                    val.as_ref(),
                                                );
                                            });
                                    }
                                });

                                ui.horizontal(|ui| {
                                    match &mut cfg.emitters[i].def.attributes.rgb.behavior {
                                        Some(ColorBehavior::Simple(Behavior::To {
                                            value,
                                            curve,
                                        })) => {
                                            let mut rgb = linear_rgb_from(*value);
                                            ui.color_edit_button_rgb(&mut rgb);
                                            *value = gamme_rgb_from(rgb);
                                            ui.separator();
                                            ui.label("Curve: ");
                                            egui::ComboBox::from_id_salt("color_simple_to_curve")
                                                .selected_text(curve.as_ref())
                                                .show_ui(ui, |ui| {
                                                    Curve::iter().for_each(|c| {
                                                        ui.selectable_value(
                                                            curve,
                                                            c.clone(),
                                                            c.as_ref(),
                                                        );
                                                    });
                                                });
                                        }
                                        Some(ColorBehavior::Simple(Behavior::Increment(val))) => {
                                            ui.label("Amount: ");
                                            let mut n = val.x;
                                            ui.add(egui::Slider::new(&mut n, -1.0..=1.0));
                                            *val = Vec3::splat(n);
                                        }
                                        _ => {}
                                    }
                                });

                                if let Some(ColorBehavior::ByChannel { red, green, blue }) =
                                    &mut cfg.emitters[i].def.attributes.rgb.behavior
                                {
                                    [("Red", red), ("Green", green), ("Blue", blue)]
                                        .into_iter()
                                        .for_each(|(name, value)| {
                                            ui.horizontal(|ui| {
                                                ui.label(format!("{name}: "));

                                                let selected = value
                                                    .as_ref()
                                                    .map_or("None", |v| {
                                                        let s: &str = v.as_ref();
                                                        s
                                                    })
                                                    .to_string();

                                                egui::ComboBox::from_id_salt(format!(
                                                    "channel_{name}_behavior"
                                                ))
                                                .selected_text(selected)
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(value, None, "None");

                                                    let val = Behavior::To {
                                                        value: 1.0,
                                                        curve: Curve::Linear,
                                                    };
                                                    ui.selectable_value(
                                                        value,
                                                        Some(val.clone()),
                                                        val.as_ref(),
                                                    );

                                                    let val = Behavior::Increment(0.0);
                                                    ui.selectable_value(
                                                        value,
                                                        Some(val.clone()),
                                                        val.as_ref(),
                                                    );
                                                });

                                                match value {
                                                    Some(Behavior::To { value, curve }) => {
                                                        ui.label("To: ");
                                                        ui.add(egui::Slider::new(
                                                            value,
                                                            -1.0..=1.0,
                                                        ));

                                                        ui.separator();
                                                        egui::ComboBox::from_id_salt(format!(
                                                            "channel_{name}_curve"
                                                        ))
                                                        .selected_text(curve.as_ref())
                                                        .show_ui(ui, |ui| {
                                                            Curve::iter().for_each(|c| {
                                                                ui.selectable_value(
                                                                    curve,
                                                                    c.clone(),
                                                                    c.as_ref(),
                                                                );
                                                            });
                                                        });
                                                    }
                                                    Some(Behavior::Increment(inc)) => {
                                                        ui.label("Amount: ");
                                                        ui.add(
                                                            egui::DragValue::new(inc).speed(0.1),
                                                        );
                                                    }
                                                    None => {}
                                                }
                                            });
                                        });
                                }
                            });

                            ui.separator();
                            egui::CollapsingHeader::new("Alpha").show(ui, |ui| {
                                let value = &mut cfg.emitters[i].def.attributes.alpha.initial;
                                ui.label("Initial: ");
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_id_salt("init_alpha")
                                        .selected_text(value.as_ref())
                                        .show_ui(ui, |ui| {
                                            let val = Value::Fixed(1.0);
                                            ui.selectable_value(value, val, val.as_ref());

                                            let val = Value::Range { min: 0.0, max: 1.0 };
                                            ui.selectable_value(value, val, val.as_ref());
                                        });

                                    if let Value::Fixed(val) = value {
                                        ui.add(egui::Slider::new(val, 0.0..=1.0));
                                    }
                                });

                                if let Value::Range { min, max } = value {
                                    ui.horizontal(|ui| {
                                        ui.label("Min: ");
                                        ui.add(egui::Slider::new(min, 0.0..=1.0));

                                        ui.separator();

                                        ui.label("Max: ");
                                        ui.add(egui::Slider::new(max, *min..=1.0));
                                    });
                                }

                                ui.separator();
                                ui.label("Behavior: ");
                                let value = &mut cfg.emitters[i].def.attributes.alpha.behavior;
                                ui.horizontal(|ui| {
                                    let selected = value
                                        .as_ref()
                                        .map_or("None", |v| {
                                            let s: &str = v.as_ref();
                                            s
                                        })
                                        .to_string();

                                    egui::ComboBox::from_id_salt("behavior_alpha")
                                        .selected_text(selected)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(value, None, "None");

                                            let val = Behavior::To {
                                                value: 1.0,
                                                curve: Curve::Linear,
                                            };
                                            ui.selectable_value(
                                                value,
                                                Some(val.clone()),
                                                val.as_ref(),
                                            );

                                            let val = Behavior::Increment(1.0);
                                            ui.selectable_value(
                                                value,
                                                Some(val.clone()),
                                                val.as_ref(),
                                            );
                                        });

                                    match value {
                                        Some(Behavior::To { value, curve }) => {
                                            ui.label("To: ");
                                            ui.add(egui::Slider::new(value, 0.0..=1.0));

                                            ui.separator();
                                            egui::ComboBox::from_id_salt("alpha_curve")
                                                .selected_text(curve.as_ref())
                                                .show_ui(ui, |ui| {
                                                    Curve::iter().for_each(|c| {
                                                        ui.selectable_value(
                                                            curve,
                                                            c.clone(),
                                                            c.as_ref(),
                                                        );
                                                    });
                                                });
                                        }
                                        Some(Behavior::Increment(inc)) => {
                                            ui.label("Amount: ");
                                            ui.add(egui::DragValue::new(inc).speed(0.1));
                                        }
                                        None => {}
                                    }
                                });
                            });
                        });
                    }
                }
            });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Fps: {fps:.0}"));
                ui.separator();
                ui.label(format!("Ms: {ms:.0}"));
                ui.separator();
                ui.label(format!("Particles: {particles_amount}"));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(egui::Slider::new(&mut state.zoom, 0.1..=10.0).text("Zoom"));
                });
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
                ui.add(egui::Slider::new(val, 0.1..=1000.0));
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

fn value_box_angle(ui: &mut egui::Ui, value: &mut Value<f32>, id: &str) {
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(value.as_ref())
            .show_ui(ui, |ui| {
                let val = Value::Fixed(0.0);
                ui.selectable_value(value, val, val.as_ref());

                let val = Value::Range { min: 0.0, max: 1.0 };
                ui.selectable_value(value, val, val.as_ref());
            });

        if let Value::Fixed(val) = value {
            let mut vv = val.to_degrees();
            ui.add(egui::Slider::new(&mut vv, 0.0..=1000.0));
            *val = vv.to_radians();
        }
    });

    if let Value::Range { min, max } = value {
        ui.horizontal(|ui| {
            ui.label("Min: ");
            let mut min_rot = min.to_degrees();
            ui.add(egui::Slider::new(&mut min_rot, -360f32..=360.0));
            *min = min_rot.to_radians();

            ui.separator();

            ui.label("Max: ");
            let mut max_rot = max.to_degrees();
            ui.add(egui::Slider::new(&mut max_rot, min_rot..=360.0));
            *max = max_rot.to_radians();
        });
    }
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

                let val = Behavior::Increment(to);
                ui.selectable_value(value, Some(val.clone()), val.as_ref());
            });

        match value {
            Some(Behavior::To { value, curve }) => {
                ui.label("To: ");
                ui.add(egui::DragValue::new(value).speed(0.1));

                ui.separator();
                egui::ComboBox::from_id_salt(format!("{id}_curve"))
                    .selected_text(curve.as_ref())
                    .show_ui(ui, |ui| {
                        Curve::iter().for_each(|c| {
                            ui.selectable_value(curve, c.clone(), c.as_ref());
                        });
                    });
            }
            Some(Behavior::Increment(inc)) => {
                ui.label("Amount: ");
                ui.add(egui::DragValue::new(inc).speed(0.1));
            }
            None => {}
        }
    });
}

fn behavior_box_angle(ui: &mut egui::Ui, value: &mut Option<Behavior<f32>>, to: f32, id: &str) {
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

                let val = Behavior::Increment(to);
                ui.selectable_value(value, Some(val.clone()), val.as_ref());
            });

        match value {
            Some(Behavior::To { value, curve }) => {
                ui.label("To: ");
                let mut rot = value.to_degrees();
                ui.add(egui::Slider::new(&mut rot, -360f32..=360.0));
                *value = rot.to_radians();

                ui.separator();
                egui::ComboBox::from_id_salt(format!("{id}_curve"))
                    .selected_text(curve.as_ref())
                    .show_ui(ui, |ui| {
                        Curve::iter().for_each(|c| {
                            ui.selectable_value(curve, c.clone(), c.as_ref());
                        });
                    });
            }
            Some(Behavior::Increment(inc)) => {
                ui.label("Amount: ");
                ui.add(egui::DragValue::new(inc).speed(0.1));
            }
            None => {}
        }
    });
}

fn linear_rgb_from(val: Vec3) -> [f32; 3] {
    Color::from(val.to_array()).as_linear().to_rgb()
}

fn gamme_rgb_from(val: [f32; 3]) -> Vec3 {
    let linear_rgb = Color::from_linear_rgb(LinearColor {
        r: val[0],
        g: val[1],
        b: val[2],
        a: 1.0,
    });
    vec3(linear_rgb.r, linear_rgb.g, linear_rgb.b)
}

struct LoadParticleConfigCmd {
    config: ParticleFxConfig,
}

impl Command for LoadParticleConfigCmd {
    fn apply(self, world: &mut World) {
        let win_size = world.resource::<Window>().size();
        if let Ok(fx_e) = world
            .query_filtered::<Entity, With<ParticleFx>>()
            .get_single(world)
        {
            world.despawn(fx_e);
        }
        let mut configs = world.resource_mut::<Particles>();
        configs.add_config("my_fx", self.config.clone());
        let component = configs.create_component("my_fx", win_size * 0.5).unwrap();
        world.spawn(component);
    }
}
