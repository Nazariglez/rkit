use rkit::{
    draw::create_draw_2d,
    ecs::ui::experimental::{
        CommandUISceneExt, UIClick, UIDragInput, UIPointerEnter, UIPointerLeave, UIPointerPressed,
        UIPointerReleased, UIScene, UIScrollInput, ui,
    },
    gfx::{self, Color},
    input::MouseButton,
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component)]
struct EventPanel;

#[derive(Component)]
struct EventStatus;

#[derive(Component)]
struct EventAction(&'static str);

#[derive(Resource, Default)]
struct LatestEvent {
    global: String,
    local: String,
    polling: String,
    scrolling: String,
}

#[derive(Resource)]
struct EventState {
    panel: Entity,
    target: Option<Entity>,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .insert_resource(LatestEvent::default())
        .on_event(record_enter)
        .on_event(record_leave)
        .on_event(record_pressed)
        .on_event(record_released)
        .on_event(record_click)
        .on_event(record_drag)
        .on_event(record_scroll)
        .on_setup(setup)
        .on_pre_update(update_layout.before(UILayoutSysSet))
        .on_update((controls, poll_pointer, show_status).chain())
        .on_render(draw)
        .run()
}

fn setup(mut commands: Commands) {
    let root = commands.spawn_ui(MainLayout, workspace());
    let children = commands.spawn_ui_children(MainLayout, root, [event_panel(), scroll_zone()]);
    let panel = children[0];
    let target = commands.spawn_ui_children(MainLayout, panel, [event_target()])[0];
    commands.insert_resource(EventState {
        panel,
        target: Some(target),
    });
}

fn workspace() -> UIScene {
    ui::column()
        .style(|style| {
            style
                .size_full()
                .align_items_center()
                .justify_content_center()
                .padding(14.0)
        })
        .style(|style| style.gap(10.0))
        .children([
            ui::text("Pointer events").style(|style| style.size(700.0, 28.0)),
            ui::text("Orange: hover, click, and drag\nBlue: wheel | C: remove/restore")
                .style(|style| style.size(700.0, 40.0)),
            ui::text("")
                .insert(EventStatus)
                .style(|style| style.size(700.0, 120.0)),
        ])
}

fn event_panel() -> UIScene {
    ui::container(UIContainer {
        bg_color: Some(Color::rgb(0.1, 0.16, 0.26)),
        border_color: Some(Color::WHITE),
        border_size: 2.0,
        corner_radius: Some(12.0),
    })
    .insert((EventPanel, EventAction("blue parent"), UIPointer::default()))
    .style(|style| {
        style
            .size(460.0, 140.0)
            .align_items_center()
            .justify_content_center()
            .padding(12.0)
    })
}

fn event_target() -> UIScene {
    ui::container(UIContainer {
        bg_color: Some(Color::ORANGE),
        border_color: Some(Color::WHITE),
        border_size: 2.0,
        corner_radius: Some(10.0),
    })
    .insert((EventAction("orange child"), UIPointer::default()))
    .on_click(local_click)
    .style(|style| {
        style
            .size(380.0, 110.0)
            .align_items_center()
            .justify_content_center()
    })
    .child(ui::text("ORANGE TARGET\nlocal click observer"))
}

fn scroll_zone() -> UIScene {
    ui::container(UIContainer {
        bg_color: Some(Color::rgb(0.1, 0.28, 0.48)),
        border_color: Some(Color::WHITE),
        border_size: 2.0,
        corner_radius: Some(12.0),
    })
    .insert((
        EventAction("blue list"),
        UIPointer::default(),
        UIScroll::vertical(),
    ))
    .style(|style| {
        style
            .size(460.0, 120.0)
            .flex_col()
            .gap(6.0)
            .padding(10.0)
            .overflow_clip()
    })
    .children((1..=6).map(|row| {
        ui::text(format!("scroll row {row}"))
            .style(|style| style.size(400.0, 28.0).flex_shrink(0.0))
    }))
}

fn update_layout(mut layout: ResMut<UILayout<MainLayout>>, window: Res<Window>) {
    layout.set_size(window.size());
}

fn controls(keyboard: Res<Keyboard>, mut commands: Commands, mut state: ResMut<EventState>) {
    if !keyboard.just_pressed(KeyCode::KeyC) {
        return;
    }

    match state.target.take() {
        Some(target) => commands.entity(target).despawn(),
        None => {
            state.target =
                Some(commands.spawn_ui_children(MainLayout, state.panel, [event_target()])[0])
        }
    }
}

fn poll_pointer(
    state: Res<EventState>,
    pointers: Query<&UIPointer>,
    scroll: Single<(&UIPointer, &UIScroll)>,
    mut latest: ResMut<LatestEvent>,
) {
    let (scroll_pointer, scroll) = scroll.into_inner();
    latest.scrolling = format!(
        "scroll poll: {:?} | offset: {:.0}/{:.0}",
        scroll_pointer.scroll(),
        scroll.offset(),
        scroll.max_offset(),
    );
    let Some(target) = state.target else {
        latest.polling = "poll: orange target removed".to_string();
        return;
    };
    let Ok(pointer) = pointers.get(target) else {
        latest.polling = "poll: waiting for restored target".to_string();
        return;
    };

    let drag = drag_phase(pointer.dragging(MouseButton::Left));
    latest.polling = format!(
        "poll: hover={} | {drag}\nenter={} leave={}\npressed={} released={} clicked={}",
        pointer.is_hover(),
        pointer.just_enter(),
        pointer.just_exit(),
        pointer.just_pressed(MouseButton::Left),
        pointer.just_released(MouseButton::Left),
        pointer.just_clicked(MouseButton::Left),
    );
}

fn show_status(latest: Res<LatestEvent>, mut status: Single<&mut UIText, With<EventStatus>>) {
    let text = format!(
        "Event: {}\nLocal observer: {}\n{}\n{}",
        empty(&latest.global),
        empty(&latest.local),
        latest.polling,
        latest.scrolling,
    );
    if status.text != text {
        status.text = text;
    }
}

fn empty(event: &str) -> &str {
    if event.is_empty() { "none" } else { event }
}

fn record_enter(
    event: On<UIPointerEnter>,
    actions: Query<&EventAction>,
    mut latest: ResMut<LatestEvent>,
) {
    record_global("enter", event.entity, &actions, &mut latest);
}

fn record_leave(
    event: On<UIPointerLeave>,
    actions: Query<&EventAction>,
    mut latest: ResMut<LatestEvent>,
) {
    record_global("leave", event.entity, &actions, &mut latest);
}

fn record_pressed(
    event: On<UIPointerPressed>,
    actions: Query<&EventAction>,
    mut latest: ResMut<LatestEvent>,
) {
    record_global("press", event.entity, &actions, &mut latest);
}

fn record_released(
    event: On<UIPointerReleased>,
    actions: Query<&EventAction>,
    mut latest: ResMut<LatestEvent>,
) {
    record_global("release", event.entity, &actions, &mut latest);
}

fn record_click(event: On<UIClick>, actions: Query<&EventAction>, mut latest: ResMut<LatestEvent>) {
    record_global("click", event.entity, &actions, &mut latest);
}

fn record_drag(
    event: On<UIDragInput>,
    actions: Query<&EventAction>,
    mut latest: ResMut<LatestEvent>,
) {
    record_global(
        drag_phase(Some(event.event)),
        event.entity,
        &actions,
        &mut latest,
    );
}

fn drag_phase(event: Option<UIDragEvent>) -> &'static str {
    match event {
        Some(UIDragEvent::Start(_)) => "drag start",
        Some(UIDragEvent::Move { .. }) => "drag move",
        Some(UIDragEvent::End(_)) => "drag end",
        None => "no drag",
    }
}

fn record_scroll(
    event: On<UIScrollInput>,
    actions: Query<&EventAction>,
    mut latest: ResMut<LatestEvent>,
) {
    record_global(
        &format!("scroll {:?}", event.delta),
        event.entity,
        &actions,
        &mut latest,
    );
}

fn local_click(event: On<UIClick>, actions: Query<&EventAction>, mut latest: ResMut<LatestEvent>) {
    let label = actions
        .get(event.entity)
        .map_or("unknown", |action| action.0);
    latest.local = format!("click: {label} ({:?})", event.entity);
    println!("local click on {:?}: {label}", event.entity);
}

fn record_global(
    kind: &str,
    entity: Entity,
    actions: &Query<&EventAction>,
    latest: &mut LatestEvent,
) {
    let label = actions.get(entity).map_or("unknown", |action| action.0);
    latest.global = format!("{kind}: {label} ({entity:?})");
    println!("global {kind}: {label} ({entity:?})");
}

fn draw(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
