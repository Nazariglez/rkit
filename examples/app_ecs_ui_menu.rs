use std::f32::consts::PI;

use rkit::{
    draw::create_draw_2d,
    ecs::ui::{CommandUISceneExt, UIClick, UIScene, ui},
    gfx::{self, Color},
    math::Vec2,
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component, Default)]
struct JumpOnClick {
    phase: f32,
}

#[derive(Component)]
struct GrowOnHover;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .on_setup(setup)
        .on_pre_update(update_layout.before(UILayoutSysSet))
        .on_update((jump_on_click, grow_on_hover).chain())
        .on_render(draw)
        .run()
}

fn setup(mut commands: Commands) {
    commands.spawn_ui(
        MainLayout,
        ui::column()
            .style(|style| {
                style
                    .size_full()
                    .align_items_center()
                    .justify_content_center()
            })
            .child(menu([button("Start"), button("Credits"), button("Exit")])),
    );
}

fn menu(buttons: impl IntoIterator<Item = UIScene>) -> UIScene {
    ui::container(UIContainer {
        bg_color: Some(Color::rgb(0.12, 0.18, 0.3)),
        corner_radius: Some(12.0),
        ..Default::default()
    })
    .style(|style| style.flex_col().gap(18.0).padding(30.0))
    .children(buttons)
}

fn button(label: &'static str) -> UIScene {
    ui::node()
        .insert(UIPointer::default())
        .on_click(move |event: On<UIClick>| {
            if event.button == MouseButton::Left {
                log::info!("Clicked {label}");
            }
        })
        .style(|style| style.size(220.0, 48.0))
        .child(
            ui::container(UIContainer {
                bg_color: Some(Color::rgb(0.2, 0.38, 0.72)),
                corner_radius: Some(8.0),
                ..Default::default()
            })
            .insert((JumpOnClick::default(), GrowOnHover))
            .style(|style| {
                style
                    .size_full()
                    .align_items_center()
                    .justify_content_center()
            })
            .child(ui::text(label)),
        )
}

fn update_layout(mut layout: ResMut<UILayout<MainLayout>>, window: Res<Window>) {
    layout.set_size(window.size());
}

fn jump_on_click(
    time: Res<Time>,
    mut nodes: Query<(&mut JumpOnClick, &mut UITransform, &ChildOf)>,
    pointers: Query<&UIPointer>,
) {
    let step = time.delta_f32() / 0.24;
    for (mut jump, mut transform, parent) in &mut nodes {
        let pointer = pointers.get(parent.parent()).unwrap();
        jump.phase = (jump.phase - step).max(0.0);
        if pointer.just_clicked(MouseButton::Left) {
            jump.phase = jump.phase.max(1.0 - jump.phase);
        }
        transform.offset.y = -6.0 * (PI * jump.phase).sin();
    }
}

fn grow_on_hover(
    time: Res<Time>,
    mut nodes: Query<(&mut UITransform, &ChildOf), With<GrowOnHover>>,
    pointers: Query<&UIPointer>,
) {
    let blend = 1.0 - (-24.0 * time.delta_f32()).exp();
    for (mut transform, parent) in &mut nodes {
        let pointer = pointers.get(parent.parent()).unwrap();
        let scale = if pointer.is_hover() { 1.1 } else { 1.0 };
        transform.scale = transform.scale.lerp(Vec2::splat(scale), blend);
    }
}

fn draw(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
