use rkit::{
    draw::create_draw_2d,
    gfx::{self, Color},
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component)]
#[require(UIPointer)]
struct Highlight(Color);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .on_setup(setup)
        .on_pre_update(update_layout.before(UILayoutSysSet))
        .on_update(highlight)
        .on_render(draw)
        .run()
}

fn setup(mut commands: Commands) {
    commands
        .spawn_ui_node(
            MainLayout,
            UIStyle::default()
                .size_full()
                .flex_row()
                .align_items_center()
                .justify_content_center()
                .gap(20.0),
        )
        .with_children(|root| {
            root.add((
                UIContainer {
                    bg_color: Some(Color::rgb(0.12, 0.18, 0.28)),
                    border_color: Some(Color::GREEN),
                    border_size: 3.0,
                    ..Default::default()
                },
                UIStyle::default()
                    .size(220.0, 220.0)
                    .flex_col()
                    .gap(8.0)
                    .padding(10.0),
            ))
            .with_children(|panel| add_content(panel, false));

            root.add((
                UIContainer {
                    bg_color: Some(Color::rgb(0.12, 0.18, 0.28)),
                    border_color: Some(Color::ORANGE),
                    border_size: 3.0,
                    ..Default::default()
                },
                UIStyle::default()
                    .size(220.0, 220.0)
                    .flex_col()
                    .gap(8.0)
                    .padding(10.0)
                    .overflow_clip(),
            ))
            .with_children(|panel| add_content(panel, false));

            root.add((
                UIContainer {
                    bg_color: Some(Color::rgb(0.16, 0.12, 0.24)),
                    border_color: Some(Color::WHITE),
                    border_size: 3.0,
                    corner_radius: Some(28.0),
                },
                UIStyle::default()
                    .size(220.0, 220.0)
                    .flex_col()
                    .gap(8.0)
                    .padding(10.0)
                    .overflow_rounded(28.0),
            ))
            .with_children(|panel| add_content(panel, true));
        });
}

fn add_content<T: Component + Copy>(
    panel: &mut SpawnUICommandBuilder<'_, '_, '_, T>,
    rounded: bool,
) {
    for index in 0..4 {
        let color = if rounded {
            Color::rgb(0.55, 0.22 + index as f32 * 0.08, 0.72)
        } else {
            Color::rgb(0.12, 0.42 + index as f32 * 0.08, 0.7)
        };
        panel.add((
            UIContainer {
                bg_color: Some(color),
                ..Default::default()
            },
            UIStyle::default()
                .size(250.0, 52.0)
                .flex_shrink(0.0)
                .margin_left(-22.0),
            Highlight(color),
        ));
    }

    panel
        .add(
            UIStyle::default()
                .size(180.0, 56.0)
                .flex_shrink(0.0)
                .overflow_clip(),
        )
        .with_children(|nested| {
            nested.add((
                UIContainer {
                    bg_color: Some(Color::ORANGE),
                    ..Default::default()
                },
                UIStyle::default()
                    .absolute()
                    .left(-24.0)
                    .top(8.0)
                    .size(240.0, 40.0),
                Highlight(Color::ORANGE),
            ));
        });
}

fn update_layout(mut layout: ResMut<UILayout<MainLayout>>, window: Res<Window>) {
    layout.set_size(window.size());
}

fn highlight(mut nodes: Query<(&mut UIContainer, &UIPointer, &Highlight)>) {
    for (mut container, pointer, highlight) in &mut nodes {
        container.bg_color = Some(if pointer.is_hover() {
            Color::GREEN
        } else {
            highlight.0
        });
    }
}

fn draw(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
