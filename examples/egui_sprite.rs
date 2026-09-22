use rkit::{
    draw::{RenderSprite, create_draw_2d_for, create_render_sprite, create_sprite},
    egui::{EguiContext, EguiPlugin},
    gfx::{self, Color, TextureFilter, TextureFormat},
    math::{Rect, Vec2, vec2},
    prelude::*,
};

#[derive(Resource)]
struct SpriteImages {
    full: egui::Image<'static>,
    left: egui::Image<'static>,
    right: egui::Image<'static>,
    straight: egui::Image<'static>,
    premultiplied: egui::Image<'static>,
    same_texture_straight: egui::Image<'static>,
    same_texture_nearest: egui::Image<'static>,
    zero_alpha: egui::Image<'static>,
    layer: RenderSprite,
    layer_image: egui::Image<'static>,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(EguiPlugin::default())
        .on_setup(setup_system)
        .on_render(draw_system)
        .run()
}

fn setup_system(mut commands: Commands, mut egui: ResMut<EguiContext>) {
    let mut pixels = vec![0; 64 * 32 * 4];
    for row in pixels.chunks_exact_mut(64 * 4) {
        for pixel in row[..32 * 4].chunks_exact_mut(4) {
            pixel.copy_from_slice(&[220, 80, 60, 255]);
        }
        for pixel in row[32 * 4..].chunks_exact_mut(4) {
            pixel.copy_from_slice(&[60, 140, 220, 255]);
        }
    }

    let sprite = create_sprite().from_bytes(&pixels, 64, 32).build().unwrap();
    let left = sprite.clone_with_frame(Rect::new(Vec2::ZERO, vec2(32.0, 32.0)));
    let right = sprite.clone_with_frame(Rect::new(vec2(32.0, 0.0), vec2(32.0, 32.0)));

    let straight_pixels = [255, 137, 137, 160].repeat(32 * 32);
    let straight = create_sprite()
        .from_bytes(&straight_pixels, 32, 32)
        .build()
        .unwrap();
    let premultiplied_pixels = [160, 40, 40, 160].repeat(32 * 32);
    let premultiplied = create_sprite()
        .from_bytes(&premultiplied_pixels, 32, 32)
        .with_format(TextureFormat::Rgba8UNorm)
        .with_premultiplied_source()
        .build()
        .unwrap();
    let same_texture_straight = create_sprite()
        .from_texture(premultiplied.texture())
        .with_sampler(premultiplied.sampler())
        .build()
        .unwrap();
    let same_texture_nearest = create_sprite()
        .from_texture(premultiplied.texture())
        .with_filter(TextureFilter::Nearest)
        .with_premultiplied_source()
        .build()
        .unwrap();
    let zero_alpha = create_sprite()
        .from_bytes(&[255, 64, 255, 0], 1, 1)
        .with_format(TextureFormat::Rgba8UNorm)
        .with_premultiplied_source()
        .build()
        .unwrap();
    let layer = create_render_sprite().with_size(64, 64).build().unwrap();

    let full = egui.add_sprite(&sprite);
    let left_image = egui.add_sprite(&left);
    let right_image = egui.add_sprite(&right);
    let straight_image = egui.add_sprite(&straight);
    let invalidated_premultiplied_image = egui.add_sprite(&premultiplied);
    let same_texture_straight_image = egui.add_sprite(&same_texture_straight);
    let same_texture_nearest_image = egui.add_sprite(&same_texture_nearest);
    let zero_alpha_image = egui.add_sprite(&zero_alpha);
    let layer_image = egui.add_sprite(&layer.sprite);

    egui.remove_sprite(&premultiplied);
    drop(invalidated_premultiplied_image);
    let premultiplied_image = egui.add_sprite(&premultiplied);

    commands.insert_resource(SpriteImages {
        full,
        left: left_image,
        right: right_image,
        straight: straight_image,
        premultiplied: premultiplied_image,
        same_texture_straight: same_texture_straight_image,
        same_texture_nearest: same_texture_nearest_image,
        zero_alpha: zero_alpha_image,
        layer,
        layer_image,
    });
}

fn render_layer(layer: &RenderSprite, elapsed: f32) {
    let mut draw = create_draw_2d_for(&layer.render_texture);
    draw.clear(Color::TRANSPARENT);
    draw.circle(20.0)
        .position(vec2(32.0 + elapsed.sin() * 12.0, 32.0))
        .fill_color(Color::rgb(0.2, 0.9, 0.8).with_alpha(0.65))
        .fill();
    draw.circle(14.0)
        .position(vec2(32.0, 32.0 + elapsed.cos() * 10.0))
        .fill_color(Color::MAGENTA.with_alpha(0.65))
        .fill();
    gfx::render_to_texture(&layer.render_texture, &draw).unwrap();
}

fn draw_system(mut egui: ResMut<EguiContext>, images: Res<SpriteImages>, time: Res<Time>) {
    render_layer(&images.layer, time.elapsed_f32());

    let draw = egui.clear(Color::BLACK).run(|ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Sprite images");
            ui.label("Full texture at its sprite display size:");
            ui.add(images.full.clone());

            ui.separator();
            ui.label("Atlas frames and native image button:");
            ui.horizontal(|ui| {
                ui.add(
                    images
                        .left
                        .clone()
                        .fit_to_exact_size(egui::vec2(96.0, 96.0)),
                );
                ui.add(
                    images
                        .right
                        .clone()
                        .fit_to_exact_size(egui::vec2(96.0, 96.0)),
                );
                ui.add(egui::ImageButton::new(
                    images
                        .left
                        .clone()
                        .fit_to_exact_size(egui::vec2(48.0, 48.0)),
                ));
            });

            ui.separator();
            ui.label("Straight and linear-PM imports use the same tinted opacity:");
            let tint = egui::Color32::from_rgba_unmultiplied(255, 220, 180, 160);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Straight");
                    ui.add(
                        images
                            .straight
                            .clone()
                            .fit_to_exact_size(egui::vec2(64.0, 64.0))
                            .bg_fill(egui::Color32::from_rgb(35, 45, 60))
                            .tint(tint),
                    );
                });
                ui.vertical(|ui| {
                    ui.label("Declared PM");
                    ui.add(
                        images
                            .premultiplied
                            .clone()
                            .fit_to_exact_size(egui::vec2(64.0, 64.0))
                            .bg_fill(egui::Color32::from_rgb(35, 45, 60))
                            .tint(tint),
                    );
                });
                ui.vertical(|ui| {
                    ui.label("Same PM texture, straight declaration");
                    ui.add(
                        images
                            .same_texture_straight
                            .clone()
                            .fit_to_exact_size(egui::vec2(64.0, 64.0))
                            .bg_fill(egui::Color32::from_rgb(35, 45, 60))
                            .tint(tint),
                    );
                });
                ui.vertical(|ui| {
                    ui.label("Same PM texture, nearest sampler");
                    ui.add(
                        images
                            .same_texture_nearest
                            .clone()
                            .fit_to_exact_size(egui::vec2(64.0, 64.0))
                            .bg_fill(egui::Color32::from_rgb(35, 45, 60))
                            .tint(tint),
                    );
                });
            });

            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Live transparent RenderSprite");
                    ui.add(
                        images
                            .layer_image
                            .clone()
                            .fit_to_exact_size(egui::vec2(96.0, 96.0))
                            .bg_fill(egui::Color32::from_rgb(35, 45, 60)),
                    );
                });
                ui.vertical(|ui| {
                    ui.label("PM zero alpha keeps additive RGB");
                    ui.add(
                        images
                            .zero_alpha
                            .clone()
                            .fit_to_exact_size(egui::vec2(64.0, 64.0))
                            .bg_fill(egui::Color32::from_rgb(35, 45, 60)),
                    );
                });
                ui.vertical(|ui| {
                    ui.label("Zero image opacity");
                    ui.add(
                        images
                            .premultiplied
                            .clone()
                            .fit_to_exact_size(egui::vec2(64.0, 64.0))
                            .bg_fill(egui::Color32::from_rgb(35, 45, 60))
                            .tint(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 0)),
                    );
                });
            });

            ui.separator();
            ui.label("Custom-painted frame with overlay:");
            let (rect, _) = ui.allocate_exact_size(egui::vec2(128.0, 128.0), egui::Sense::hover());
            images.left.paint_at(ui, rect);
            ui.painter()
                .rect_filled(rect.shrink(24.0), 4.0, egui::Color32::from_white_alpha(96));
        });
    });

    gfx::render_to_frame(&draw).unwrap();
}
