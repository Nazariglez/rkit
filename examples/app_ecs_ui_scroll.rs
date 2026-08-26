use rkit::{
    draw::create_draw_2d,
    gfx::{self, Color},
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component)]
struct MainScroll {
    compact: bool,
    rounded: bool,
}

#[derive(Component)]
#[require(UIPointer)]
struct Row(Color);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .on_setup(setup)
        .on_pre_update(update_layout.before(UILayoutSysSet))
        .on_update((controls, highlight))
        .on_render(draw)
        .run()
}

fn setup(mut commands: Commands) {
    commands
        .spawn_ui_node(
            MainLayout,
            UIStyle::default()
                .size_full()
                .flex_col()
                .align_items_center()
                .justify_content_center()
                .gap(12.0),
        )
        .with_children(|root| {
            root.add((
                UIText {
                    text: "Wheel to scroll | 1 top | 2 middle | 3 bottom | 4 resize | Space round"
                        .to_string(),
                    size: 18.0,
                    ..Default::default()
                },
                UIStyle::default().size(620.0, 28.0),
            ));

            root.add((
                UIContainer {
                    bg_color: Some(Color::rgb(0.1, 0.14, 0.22)),
                    border_color: Some(Color::WHITE),
                    border_size: 3.0,
                    ..Default::default()
                },
                UIStyle::default()
                    .size(320.0, 260.0)
                    .flex_col()
                    .gap(6.0)
                    .padding(10.0),
                UIScroll::vertical(),
                MainScroll {
                    compact: false,
                    rounded: false,
                },
            ))
            .with_children(|list| {
                for index in 0..4 {
                    add_row(list, index, false);
                }

                list.add((
                    UIContainer {
                        bg_color: Some(Color::rgb(0.2, 0.12, 0.18)),
                        border_color: Some(Color::ORANGE),
                        border_size: 2.0,
                        corner_radius: Some(14.0),
                    },
                    UIStyle::default()
                        .size(280.0, 112.0)
                        .flex_col()
                        .gap(4.0)
                        .padding(6.0)
                        .flex_shrink(0.0)
                        .overflow_rounded(14.0),
                    UIScroll::vertical(),
                ))
                .with_children(|inner| {
                    for index in 0..6 {
                        add_row(inner, index, true);
                    }
                });

                for index in 4..10 {
                    add_row(list, index, false);
                }
            });
        });
}

fn add_row<T: Component + Copy>(
    list: &mut SpawnUICommandBuilder<'_, '_, '_, T>,
    index: usize,
    inner: bool,
) {
    let color = if inner {
        Color::rgb(0.58, 0.24 + index as f32 * 0.04, 0.2)
    } else {
        Color::rgb(0.14, 0.3 + index as f32 * 0.035, 0.62)
    };
    list.add((
        UIContainer {
            bg_color: Some(color),
            corner_radius: Some(8.0),
            ..Default::default()
        },
        UIStyle::default()
            .size(
                if inner { 250.0 } else { 280.0 },
                if inner { 36.0 } else { 48.0 },
            )
            .flex_shrink(0.0),
        Row(color),
    ));
}

fn update_layout(mut layout: ResMut<UILayout<MainLayout>>, window: Res<Window>) {
    layout.set_size(window.size());
}

fn controls(
    keyboard: Res<Keyboard>,
    mut view: Single<(
        &mut UIScroll,
        &mut UIStyle,
        &mut UIContainer,
        &mut MainScroll,
    )>,
) {
    let (scroll, style, container, state) = &mut *view;
    if keyboard.just_pressed(KeyCode::Digit1) {
        scroll.scroll_to(0.0);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        let middle = scroll.max_offset() * 0.5;
        scroll.scroll_to(middle);
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        let bottom = scroll.max_offset();
        scroll.scroll_to(bottom);
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        state.compact = !state.compact;
        style.height = px(if state.compact { 170.0 } else { 260.0 });
    }
    if keyboard.just_pressed(KeyCode::Space) {
        state.rounded = !state.rounded;
        if state.rounded {
            style.overflow = UIOverflow::Rounded(24.0);
            container.corner_radius = Some(24.0);
        } else {
            style.overflow = UIOverflow::Visible;
            container.corner_radius = None;
        }
    }
}

fn highlight(mut rows: Query<(&mut UIContainer, &UIPointer, &Row)>) {
    for (mut container, pointer, row) in &mut rows {
        container.bg_color = Some(if pointer.is_hover() {
            Color::GREEN
        } else {
            row.0
        });
    }
}

fn draw(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
