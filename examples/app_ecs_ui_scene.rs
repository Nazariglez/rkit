use rkit::{
    draw::create_draw_2d,
    ecs::ui::experimental::{CommandUISceneExt, UIScene, ui},
    gfx::{self, Color},
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct SceneStatus;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Choice {
    Play,
    Options,
    Quit,
}

impl Choice {
    fn label(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Options => "Options",
            Self::Quit => "Quit",
        }
    }

    fn key(self) -> u8 {
        match self {
            Self::Play => 1,
            Self::Options => 2,
            Self::Quit => 3,
        }
    }
}

#[derive(Component, Clone, Copy)]
struct ButtonColor(Color);

#[derive(Component, Clone, Copy)]
struct SampleRow(u8);

#[derive(Resource)]
struct MenuState {
    root: Entity,
    selected: Choice,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .on_setup(setup)
        .on_pre_update(update_layout.before(UILayoutSysSet))
        .on_update((controls, update_menu).chain())
        .on_render(draw)
        .run()
}

fn setup(mut commands: Commands) {
    let root = commands.spawn_ui(MainLayout, menu());
    commands.insert_resource(MenuState {
        root,
        selected: Choice::Play,
    });
}

fn menu() -> UIScene {
    ui::column()
        .insert(MenuRoot)
        .style(|style| {
            style
                .size_full()
                .align_items_center()
                .justify_content_center()
        })
        .style(|style| style.gap(14.0).padding(20.0))
        .children([
            ui::text("UIScene composition").style(|style| style.size(320.0, 28.0)),
            ui::container(UIContainer {
                bg_color: Some(Color::rgb(0.12, 0.18, 0.3)),
                border_color: Some(Color::WHITE),
                border_size: 2.0,
                corner_radius: Some(12.0),
            })
            .style(|style| style.size(360.0, 230.0).flex_col().padding(12.0))
            .style(|style| style.gap(10.0).align_items_center())
            .children([
                menu_buttons(),
                ui::text("")
                    .insert(SceneStatus)
                    .style(|style| style.size(320.0, 44.0)),
            ]),
            sample_rows(),
        ])
}

fn menu_buttons() -> UIScene {
    ui::column()
        .style(|style| style.gap(6.0).align_items_center())
        .children([
            menu_button(Choice::Play),
            menu_button(Choice::Options),
            menu_button(Choice::Quit),
        ])
}

fn menu_button(choice: Choice) -> UIScene {
    let color = Color::rgb(0.2, 0.38, 0.72);
    ui::container(UIContainer {
        bg_color: Some(color),
        corner_radius: Some(8.0),
        ..Default::default()
    })
    .insert((choice, ButtonColor(color)))
    .style(|style| style.size(320.0, 38.0).padding_x(10.0))
    .style(|style| style.align_items_center().justify_content_center())
    .child(ui::text(format!("{}  {}", choice.key(), choice.label())))
}

fn sample_rows() -> UIScene {
    let rows = (1..=3)
        .map(|number| {
            ui::container(UIContainer {
                bg_color: Some(Color::rgb(0.16, 0.22, 0.36)),
                ..Default::default()
            })
            .insert(SampleRow(number))
            .style(|style| style.size(180.0, 26.0).padding_x(6.0))
            .child(ui::text(format!("typed row {number}")))
        })
        .collect::<Vec<_>>();

    ui::row().style(|style| style.gap(6.0)).children(rows)
}

fn update_layout(mut layout: ResMut<UILayout<MainLayout>>, window: Res<Window>) {
    layout.set_size(window.size());
}

fn controls(keyboard: Res<Keyboard>, mut state: ResMut<MenuState>) {
    for (key, choice) in [
        (KeyCode::Digit1, Choice::Play),
        (KeyCode::Digit2, Choice::Options),
        (KeyCode::Digit3, Choice::Quit),
    ] {
        if keyboard.just_pressed(key) {
            state.selected = choice;
        }
    }
}

fn update_menu(
    state: Res<MenuState>,
    mut buttons: Query<(&Choice, &ButtonColor, &mut UIContainer)>,
    mut status: Single<&mut UIText, With<SceneStatus>>,
    rows: Query<&SampleRow>,
    roots: Query<&Children, With<MenuRoot>>,
) {
    if !state.is_changed() {
        return;
    }

    for (choice, color, mut container) in &mut buttons {
        container.bg_color = Some(if *choice == state.selected {
            Color::GREEN
        } else {
            color.0
        });
    }

    let children = roots.get(state.root).unwrap();
    let row_ids = rows.iter().map(|row| row.0).collect::<Vec<_>>();
    status.text = format!(
        "Selected: {} | Menu children: {}\nTyped rows: {row_ids:?}",
        state.selected.label(),
        children.len(),
    );
}

fn draw(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
