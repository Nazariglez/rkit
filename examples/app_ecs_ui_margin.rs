use rkit::{
    draw::{Camera2D, ScreenMode},
    math::{Vec2, vec2},
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct FixedLayout;

#[derive(Component, Clone, Copy)]
struct PixelLayout;

#[derive(Component)]
struct ParentNode;

#[derive(Component)]
struct MarginNode;

#[derive(Component)]
struct NestedNode;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::headless())
        .add_plugin(UILayoutPlugin::<FixedLayout>::default())
        .add_plugin(UILayoutPlugin::<PixelLayout>::default())
        .on_setup(setup_system)
        .on_post_update(assert_layout_system.after(UILayoutSysSet))
        .run()
}

fn setup_system(
    mut cmds: Commands,
    mut fixed_layout: ResMut<UILayout<FixedLayout>>,
    mut pixel_layout: ResMut<UILayout<PixelLayout>>,
) {
    let size = vec2(80.0, 60.0);
    fixed_layout.set_size(size);

    let mut camera = Camera2D::new(size, ScreenMode::Normal);
    camera.set_position(size * 0.5);
    camera.set_pixel_perfect(true);
    camera.update();
    pixel_layout.set_camera(&camera);

    spawn_tree(&mut cmds, FixedLayout);
    spawn_tree(&mut cmds, PixelLayout);
}

fn spawn_tree<T: Component + Copy>(cmds: &mut Commands, layout: T) {
    cmds.spawn_ui_node(
        layout,
        (
            UIStyle::default()
                .size(80.0, 60.0)
                .padding_x(5.0)
                .padding_y(4.0),
            ParentNode,
        ),
    )
    .with_children(|cmd| {
        cmd.add((
            UIStyle::default()
                .size(55.0, 30.0)
                .margin_left(15.0)
                .margin_top(20.0)
                .padding(2.0),
            MarginNode,
        ))
        .with_children(|cmd| {
            cmd.add((UIStyle::default().size(45.0, 20.0), NestedNode));
        });
    });
}

fn assert_layout_system(
    mut cmds: Commands,
    fixed_parent: Single<&UINode, (With<FixedLayout>, With<ParentNode>)>,
    fixed_child: Single<&UINode, (With<FixedLayout>, With<MarginNode>)>,
    fixed_nested: Single<&UINode, (With<FixedLayout>, With<NestedNode>)>,
    pixel_parent: Single<&UINode, (With<PixelLayout>, With<ParentNode>)>,
    pixel_child: Single<&UINode, (With<PixelLayout>, With<MarginNode>)>,
    pixel_nested: Single<&UINode, (With<PixelLayout>, With<NestedNode>)>,
) {
    assert_tree(*fixed_parent, *fixed_child, *fixed_nested);
    assert_tree(*pixel_parent, *pixel_child, *pixel_nested);
    cmds.exit();
}

fn assert_tree(parent: &UINode, child: &UINode, nested: &UINode) {
    assert_eq!(parent.position(), Vec2::ZERO);
    assert_eq!(parent.size(), vec2(80.0, 60.0));
    assert_eq!(parent.bounds().origin, Vec2::ZERO);
    assert_eq!(parent.bounds().size, vec2(80.0, 60.0));

    assert_eq!(child.position(), vec2(20.0, 24.0));
    assert_eq!(child.size(), vec2(55.0, 30.0));
    assert_eq!(nested.position(), vec2(2.0, 2.0));
    assert_eq!(nested.size(), vec2(45.0, 20.0));

    let parent_max = parent.bounds().max();
    let child_max = child.position() + child.size();
    let nested_max = child.position() + nested.position() + nested.size();
    assert_eq!(child_max, vec2(75.0, 54.0));
    assert_eq!(nested_max, vec2(67.0, 46.0));
    assert!(child_max.cmple(parent_max).all());
    assert!(nested_max.cmple(parent_max).all());
}
