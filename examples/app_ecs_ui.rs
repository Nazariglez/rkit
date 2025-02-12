use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::prelude::*;

/// Identify the layout and the entities that belongs to it
#[derive(Component, Clone, Copy)]
struct MainLayout;

/// Allows an entity to be draggable
#[derive(Component)]
#[require(UIPointer)]
struct Draggable;

/// Allows an entity to be draggable
#[derive(Component)]
#[require(UIPointer)]
struct Highlight {
    color: Color,
    base: Color,
}

#[derive(Component)]
#[require(UIPointer)]
struct AlphaOnScroll;

/// Display the entity rotatin at N speed
#[derive(Component)]
struct Rotate(f32);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .add_systems(OnSetup, setup_system)
        .add_systems(OnPreUpdate, update_layout_system)
        .add_systems(
            OnUpdate,
            (alpha_system, rotation_system, highlight_system, drag_system),
        )
        .add_systems(OnRender, draw_system)
        .run()
}

fn setup_system(mut cmds: Commands) {
    cmds.spawn_ui_node(
        MainLayout,
        (
            UIContainer {
                bg_color: Some(Color::WHITE),
            },
            UIStyle::default()
                .flex_row()
                .size_full()
                .align_items_center()
                .gap_x(20.0)
                .justify_content_center(),
            AlphaOnScroll,
        ),
    )
    .with_children(|cmd| {
        cmd.add(((
            UIContainer {
                bg_color: Some(Color::ORANGE),
            },
            UIStyle::default()
                .align_items_center()
                .justify_content_center()
                .size(200.0, 100.0),
            Draggable,
        ),))
            .with_children(|cmd| {
                cmd.add((
                    UIContainer {
                        bg_color: Some(Color::BLUE),
                    },
                    UIStyle::default().size(20.0, 50.0),
                    Rotate(50.0),
                    // UIPointer::default(),
                ));
            });

        cmd.add(((
            UIContainer {
                bg_color: Some(Color::RED),
            },
            UIStyle::default().size(100.0, 100.0),
            Highlight {
                color: Color::GREEN,
                base: Color::RED,
            },
        ),));
    });
}

/// we need to set the layout's size or pass a camera to know the root's size
fn update_layout_system(mut layout: ResMut<UILayout<MainLayout>>, win: Res<Window>) {
    layout.set_size(win.size());
}

fn drag_system(mut query: Query<(&mut UITransform, &UIPointer), With<Draggable>>) {
    query.iter_mut().for_each(|(mut transform, pointer)| {
        if let Some(UIDragEvent::Move { delta, .. }) = pointer.dragging(MouseButton::Left) {
            transform.offset = transform.offset.lerp(transform.offset + delta, 0.99);
        }
    });
}

fn highlight_system(mut query: Query<(&mut UIContainer, &UIPointer, &Highlight)>) {
    for (mut container, pointer, highlight) in &mut query {
        if pointer.just_enter() {
            container.bg_color = Some(highlight.color);
        } else if pointer.just_exit() {
            container.bg_color = Some(highlight.base);
        }
    }
}

fn rotation_system(mut query: Query<(&mut UITransform, &Rotate)>, time: Res<Time>) {
    query.iter_mut().for_each(|(mut transform, rot)| {
        transform.rotation += rot.0.to_radians() * time.delta_f32();
    });
}

fn alpha_system(
    mut query: Query<(&mut UIStyle, &UIPointer), With<AlphaOnScroll>>,
    time: Res<Time>,
) {
    query.iter_mut().for_each(|(mut style, pointer)| {
        if let Some(scroll) = pointer.scroll() {
            style.opacity = (style.opacity + scroll.y * time.delta_f32()).clamp(0.0, 1.0);
        }
    });
}

fn draw_system(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
