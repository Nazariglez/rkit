use rkit::draw::create_draw_2d;
use rkit::gfx::{self, Color};
use rkit::prelude::*;

#[derive(Component, Clone, Copy)]
struct MainLayout;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .add_systems(OnSetup, setup_system)
        .add_systems(OnPreUpdate, update_layout_system)
        .add_systems(OnUpdate, (rot_system, on_hover_system, drag_system))
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
            UIPointer::default(),
        ),))
            .with_children(|cmd| {
                cmd.add((
                    UIContainer {
                        bg_color: Some(Color::BLUE),
                    },
                    UIStyle::default().size(20.0, 50.0),
                ));
            });

        cmd.add(((
            UIContainer {
                bg_color: Some(Color::RED),
            },
            UIStyle::default().size(100.0, 100.0),
            RotEffect(50.0),
            UIPointer::default(),
        ),));
    });
}

fn update_layout_system(mut layout: ResMut<UILayout<MainLayout>>, win: Res<Window>) {
    layout.set_size(win.size());
}

#[derive(Component)]
struct Draggable;

// TODO: stopPropagation?

fn drag_system(mut query: Query<(&mut UITransform, &UIPointer), With<Draggable>>, time: Res<Time>) {
    query.iter_mut().for_each(|(mut transform, pointer)| {
        match pointer.dragging(MouseButton::Left) {
            Some(evt) => match evt {
                UIDragEvent::Start(vec2) => println!("drag start"),
                UIDragEvent::Move {
                    start,
                    pos,
                    parent_delta,
                } => {
                    transform.offset = transform
                        .offset
                        .lerp(transform.offset + parent_delta, time.delta_f32());
                }
                UIDragEvent::End(vec2) => println!("drag end"),
            },
            _ => {}
        }
    });
}

fn on_hover_system(mut query: Query<(&mut UIContainer, &UIPointer), With<RotEffect>>) {
    for (mut container, pointer) in &mut query {
        if pointer.just_enter() {
            container.bg_color = Some(Color::GREEN);
        } else if pointer.just_exit() {
            container.bg_color = Some(Color::RED);
        }
        // if pointer.just_released(MouseButton::Left) {
        //     container.bg_color = Some(Color::PINK);
        // }
        //
        // println!("{pointer:?}");
        if let Some(scroll) = pointer.scroll() {
            println!("scrolling {scroll}");
        }
    }
}

#[derive(Component)]
struct RotEffect(f32);

fn rot_system(mut query: Query<(&mut UITransform, &RotEffect)>, time: Res<Time>) {
    query.iter_mut().for_each(|(mut transform, rot)| {
        transform.rotation += rot.0.to_radians() * time.delta_f32();
    });
}

fn draw_system(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
