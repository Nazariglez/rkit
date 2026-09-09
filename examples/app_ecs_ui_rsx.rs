use rkit::{
    draw::create_draw_2d,
    ecs::ui::{CommandUISceneExt, UIClick, UIScene, rsx, ui, ui_widget},
    gfx::{self, Color},
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component, Clone, Copy)]
enum Action {
    Play,
    Settings,
    Quit,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Settings => "Settings",
            Self::Quit => "Quit",
        }
    }
}

#[derive(Component)]
struct ActionStatus;

#[derive(Resource, Default)]
struct LatestAction(Option<Action>);

#[ui_widget(Panel)]
fn panel(
    title: String,
    color: Option<Color>,
    children: impl IntoIterator<Item = UIScene>,
) -> UIScene {
    let color = color.unwrap_or(Color::rgb(0.12, 0.18, 0.3));
    ui::container(UIContainer {
        bg_color: Some(color),
        border_color: Some(Color::WHITE),
        border_size: 2.0,
        corner_radius: Some(12.0),
    })
    .style(|style| style.flex_col().gap(10.0).padding(14.0))
    .children([
        ui::text(title).style(|style| style.size(340.0, 26.0)),
        ui::column()
            .style(|style| style.flex_col().gap(8.0))
            .children(children),
    ])
}

#[ui_widget(Label)]
fn label(text: String) -> UIScene {
    ui::text(text).style(|style| style.size(340.0, 22.0))
}

#[ui_widget(PanelContent)]
fn panel_content(
    title: String,
    color: Option<Color>,
    subtitle: Option<String>,
    children: impl IntoIterator<Item = UIScene>,
) -> UIScene {
    rsx! {
        <Panel title={title} color={color}>
            {subtitle.map(label)}
            {children}
        </Panel>
    }
}

#[ui_widget(BorrowedLabel)]
fn borrowed_label(text: &str) -> UIScene {
    ui::text(text).style(|style| style.size(340.0, 22.0))
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .insert_resource(LatestAction::default())
        .on_event(select_action)
        .on_setup(setup)
        .on_pre_update(update_layout.before(UILayoutSysSet))
        .on_update(update_status)
        .on_render(draw)
        .run()
}

fn setup(mut commands: Commands) {
    commands.spawn_ui(MainLayout, screen());
}

fn screen() -> UIScene {
    let construction_note =
        String::from("This label resolves borrowed text during scene construction.");
    let welcome_color = Some(Color::rgb(0.12, 0.18, 0.3));
    let welcome_subtitle = Some(String::from(
        "Optional owned text becomes a child when present.",
    ));
    let action_hint = Some("Choose an action");
    let actions = [Action::Play, Action::Settings, Action::Quit];

    rsx! {
        <column ui:style={|style| style
            .size_full()
            .align_items_center()
            .justify_content_center()
            .padding(20.0)}>
            <PanelContent
                title={"RSX UI scenes"}
                color={welcome_color}
                subtitle={welcome_subtitle}
            >
                <BorrowedLabel text={construction_note.as_str()}/>
                <Label text={"The wrapper forwards optional color without branching around its children."}/>
            </PanelContent>
            <Panel
                title={"Actions"}
                color={Color::rgb(0.16, 0.28, 0.48)}
                ui:style={|style| style.margin_top(14.0)}
            >
                <Label text={"Each button shares one click observer."}/>
                {action_hint.map(|hint| label(hint.to_owned()))}
                {actions.into_iter().map(action_button)}
                <Label
                    text={"Click an action button"}
                    ui:insert={ActionStatus}
                />
            </Panel>
        </column>
    }
}

fn action_button(action: Action) -> UIScene {
    rsx! {
        <container
            bg_color={Color::rgb(0.2, 0.38, 0.72)}
            corner_radius={8.0}
            ui:insert={(action, UIPointer::default())}
            ui:style={|style| style
                .size(340.0, 38.0)
                .align_items_center()
                .justify_content_center()}
        >
            <Label text={action.label()}/>
        </container>
    }
}

fn select_action(event: On<UIClick>, actions: Query<&Action>, mut latest: ResMut<LatestAction>) {
    latest.0 = actions.get(event.entity).ok().copied();
}

fn update_status(latest: Res<LatestAction>, mut status: Single<&mut UIText, With<ActionStatus>>) {
    if !latest.is_changed() {
        return;
    }

    status.text = match latest.0 {
        Some(action) => format!("Selected: {}", action.label()),
        None => "Click an action button".to_string(),
    };
}

fn update_layout(mut layout: ResMut<UILayout<MainLayout>>, window: Res<Window>) {
    layout.set_size(window.size());
}

fn draw(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
