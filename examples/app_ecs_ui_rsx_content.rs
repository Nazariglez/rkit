use rkit::{
    draw::{self, Font, RichTextLayout, Sprite, create_draw_2d, text},
    ecs::ui::{CommandUISceneExt, UIScene, rsx},
    gfx::{self, Color},
    math::vec2,
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .on_setup(setup)
        .on_pre_update(update_layout.before(UILayoutSysSet))
        .on_render(draw)
        .run()
}

fn setup(mut commands: Commands) {
    let font = draw::create_font(include_bytes!("./assets/Ubuntu-B.ttf"))
        .build()
        .unwrap();
    let sprite = draw::create_sprite()
        .from_image(include_bytes!("./assets/ferris.png"))
        .build()
        .unwrap();
    let layout = text::rich_text("[color:#73eff7]Prepared rich text[/color]\nuses the same scene.")
        .font(&font)
        .size(24.0)
        .layout()
        .unwrap();

    commands.spawn_ui(MainLayout, content(font, sprite, layout));
}

fn content(font: Font, sprite: Sprite, layout: RichTextLayout) -> UIScene {
    rsx! {
        <column ui:style={|style| style
            .size_full()
            .align_items_center()
            .justify_content_center()
            .padding(24.0)}>
            <container
                bg_color={Color::rgb(0.1, 0.16, 0.28)}
                border_color={Color::rgb(0.3, 0.7, 0.9)}
                corner_radius={12.0}
                ui:style={|style| style
                    .size(560.0, 300.0)
                    .flex_col()
                    .gap(20.0)
                    .padding(24.0)}
            >
                <text font={font} color={Color::WHITE} size={28.0}>
                    {"Text, image, and rich text"}
                </text>
                <row ui:style={|style| style
                    .gap(20.0)
                    .align_items_center()}>
                    <image sprite={sprite} tint={Color::WHITE} ui:style={|style| style.size(120.0, 120.0)}/>
                    <rich_text
                        layout={layout}
                        shadow_color={Color::BLACK}
                        shadow_offset={vec2(2.0, 2.0)}
                    />
                </row>
            </container>
        </column>
    }
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
