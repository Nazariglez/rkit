use rkit::{
    draw::create_draw_2d,
    ecs::ui::{CommandUISceneExt, UIClick, UIScene, rsx, ui, ui_widget},
    gfx::{self, Color},
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component)]
struct TextLink {
    text: Entity,
    label: String,
}

#[derive(Component, Default)]
struct ClickCount(u32);

#[derive(Component, Clone, Copy)]
enum WidgetRole {
    Button,
    Text,
}

#[derive(Component, Clone, Copy)]
struct WidgetInstance(u8);

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .on_event(update_linked_text)
        .on_setup(setup)
        .on_pre_update(update_layout.before(UILayoutSysSet))
        .on_render(draw)
        .run()
}

fn setup(mut commands: Commands) {
    commands.spawn_ui(MainLayout, screen());
}

fn screen() -> UIScene {
    rsx! {
        <column ui:style={|style| style
            .size_full()
            .align_items_center()
            .justify_content_center()
            .gap(16.0)
            .padding(24.0)}>
            <text size={30.0}>{"Linked widget instances"}</text>
            <text size={20.0}>{"Click either button to update only its internal text."}</text>
            <row ui:style={|style| style.gap(16.0)}>
                <LinkedButton instance={1} label={"Blue"} color={Color::rgb(0.2, 0.38, 0.72)}/>
                <LinkedButton instance={2} label={"Green"} color={Color::rgb(0.2, 0.62, 0.42)}/>
            </row>
        </column>
    }
}

#[ui_widget(LinkedButton)]
fn linked_button(instance: u8, label: String, color: Color) -> UIScene {
    ui::with_entities(move |scope| {
        let text = scope.reserve();

        rsx! {
            <container
                bg_color={color}
                corner_radius={8.0}
                ui:insert={(
                    WidgetRole::Button,
                    WidgetInstance(instance),
                    TextLink {
                        text,
                        label: label.clone(),
                    },
                    ClickCount::default(),
                    UIPointer::default(),
                )}
                ui:style={|style| style
                    .size(220.0, 72.0)
                    .align_items_center()
                    .justify_content_center()}
            >
                <text
                    size={16.0}
                    ui:entity={text}
                    ui:insert={(WidgetRole::Text, WidgetInstance(instance))}
                >
                    {format!("{label} #{instance}: 0")}
                </text>
            </container>
        }
    })
}

fn update_linked_text(
    event: On<UIClick>,
    mut buttons: Query<(&TextLink, &WidgetInstance, &mut ClickCount)>,
    mut texts: Query<&mut UIText>,
) {
    let Ok((link, instance, mut clicks)) = buttons.get_mut(event.entity) else {
        return;
    };
    let Ok(mut text) = texts.get_mut(link.text) else {
        return;
    };

    clicks.0 += 1;
    text.text = format!("{} #{}: {}", link.label, instance.0, clicks.0);
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
