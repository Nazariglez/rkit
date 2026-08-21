use rkit::{
    draw::{self, RichTextLayout, TextIcons, create_draw_2d, text},
    gfx::{self, Color, TextureFilter},
    math::{Vec2, vec2},
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component)]
struct Intrinsic;

#[derive(Component)]
struct FixedWidth;

#[derive(Resource)]
struct ExampleState {
    replacement: Option<RichTextLayout>,
    initial_size: Vec2,
    replacement_size: Vec2,
    fixed_size: Vec2,
    validated: bool,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .on_setup(setup_system)
        .on_post_update(assert_layout_system.after(UILayoutSysSet))
        .on_render(draw_system)
        .run()
}

fn setup_system(mut cmds: Commands, mut ui: ResMut<UILayout<MainLayout>>) {
    ui.set_size(vec2(640.0, 360.0));

    let icon = draw::create_sprite()
        .from_image(include_bytes!("./assets/text_icon_pixel.png"))
        .with_filter(TextureFilter::Nearest)
        .build()
        .unwrap();
    let icons = TextIcons::new([("pixel", icon)]).unwrap();

    let initial = text::rich_text("[color:#73eff7]Intrinsic[/color] [icon:pixel] size.")
        .icons(&icons)
        .size(20.0)
        .layout()
        .unwrap();
    let replacement = text::rich_text("[color:#ff7070]Bigger[/color] [icon:pixel]")
        .icons(&icons)
        .size(28.0)
        .layout()
        .unwrap();
    let fixed = text::rich_text(
        "[color:#ffe066]The node is wider[/color] than this [icon:pixel] snapshot.",
    )
    .icons(&icons)
    .size(18.0)
    .layout()
    .unwrap();

    let state = ExampleState {
        initial_size: initial.size(),
        replacement_size: replacement.size(),
        fixed_size: fixed.size(),
        replacement: Some(replacement),
        validated: false,
    };
    drop(icons);

    cmds.insert_resource(state);
    cmds.spawn_ui_node(
        MainLayout,
        (
            UIStyle::default()
                .size_full()
                .flex_col()
                .align_items_start()
                .gap_y(24.0)
                .padding(32.0),
            UIContainer::default(),
        ),
    )
    .with_children(|cmd| {
        cmd.add((UIRichText::new(initial), UIStyle::default(), Intrinsic));
        cmd.add((
            UIRichText::new(fixed),
            UIStyle::default().width(520.0),
            FixedWidth,
        ));
    });
}

fn assert_layout_system(
    mut cmds: Commands,
    mut state: ResMut<ExampleState>,
    intrinsic: Single<(Entity, &UINode), With<Intrinsic>>,
    fixed: Single<&UINode, With<FixedWidth>>,
) {
    if state.validated {
        return;
    }

    let (entity, node) = *intrinsic;
    if let Some(replacement) = state.replacement.take() {
        assert_size(node.size(), state.initial_size);
        assert!((fixed.size().x - 520.0).abs() < 1.0);
        assert!((fixed.size().y - state.fixed_size.y).abs() < 1.0);
        cmds.entity(entity).insert(UIRichText::new(replacement));
        return;
    }

    assert_size(node.size(), state.replacement_size);
    state.validated = true;
}

fn assert_size(actual: Vec2, expected: Vec2) {
    assert!(
        (actual.x - expected.x).abs() < 1.0,
        "width: expected {}, got {}",
        expected.x,
        actual.x
    );
    assert!(
        (actual.y - expected.y).abs() < 1.0,
        "height: expected {}, got {}",
        expected.y,
        actual.y
    );
}

fn draw_system(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.06, 0.07, 0.1));
    draw_ui_layout::<MainLayout>(&mut draw, world);
    gfx::render_to_frame(&draw).unwrap();
}
