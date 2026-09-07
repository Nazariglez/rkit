use rkit::{
    draw::{Draw2D, DrawStats, create_draw_2d},
    ecs::ui::{
        CommandUISceneExt, UIMeasure, UIMeasureInput, UIRuntimeError, UIScene,
        plugin::UILayoutUpdateEvent, ui,
    },
    gfx::{self, Color},
    math::{Vec2, vec2},
    prelude::*,
};

#[derive(Component, Clone, Copy)]
struct MainLayout;

#[derive(Component)]
struct FixedWaveform;

#[derive(Component)]
struct IntrinsicWaveform;

#[derive(Component)]
struct ChartStatus;

#[derive(Component)]
struct Waveform {
    samples: Vec<f32>,
    color: Color,
    invalid_measure: bool,
}

#[derive(Resource, Default)]
struct ChartEvidence {
    draw: DrawStats,
    invalid_measures: usize,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .add_plugin(UILayoutPlugin::<MainLayout>::default())
        .insert_resource(ChartEvidence::default())
        .on_event(record_invalid_measure)
        .on_setup(setup)
        .on_pre_update(update_layout.before(UILayoutSysSet))
        .on_update((controls, count_layout_updates, show_status).chain())
        .on_render(draw)
        .run()
}

fn setup(mut commands: Commands) {
    commands.spawn_ui(MainLayout, workspace());
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
            ui::text("Custom rendering and measurement").style(|style| style.size(700.0, 28.0)),
            ui::text("1: samples | I: invalid | R: restore\nThe panel clips the orange chart.")
                .style(|style| style.size(700.0, 40.0)),
            ui::text("")
                .insert(ChartStatus)
                .style(|style| style.size(700.0, 60.0)),
            chart_panel().children([fixed_waveform(), intrinsic_waveform()]),
        ])
}

fn chart_panel() -> UIScene {
    ui::container(UIContainer {
        bg_color: Some(Color::rgb(0.08, 0.13, 0.22)),
        border_color: Some(Color::WHITE),
        border_size: 2.0,
        corner_radius: Some(16.0),
    })
    .style(|style| {
        style
            .size(440.0, 210.0)
            .flex_col()
            .align_items_start()
            .gap(18.0)
            .padding(14.0)
            .overflow_rounded(16.0)
    })
}

fn fixed_waveform() -> UIScene {
    ui::node()
        .insert((
            FixedWaveform,
            Waveform {
                samples: samples(18),
                color: Color::ORANGE,
                invalid_measure: false,
            },
            UIRender::run::<(&Waveform, &UINode), _>(render_waveform),
        ))
        .style(|style| style.size(500.0, 68.0).flex_shrink(0.0))
}

fn intrinsic_waveform() -> UIScene {
    ui::node()
        .insert((
            IntrinsicWaveform,
            Waveform {
                samples: samples(10),
                color: Color::GREEN,
                invalid_measure: false,
            },
            UIRender::run::<(&Waveform, &UINode), _>(render_waveform),
            UIMeasure::run::<&Waveform, _>(measure_waveform),
        ))
        .style(|style| style.align_self_flex_start().flex_shrink(0.0))
}

fn samples(count: usize) -> Vec<f32> {
    (0..count)
        .map(|index| ((index as f32 * 0.7).sin() * 0.65) + ((index as f32 * 1.9).sin() * 0.2))
        .collect()
}

fn render_waveform(draw: &mut Draw2D, (waveform, node): (&Waveform, &UINode)) {
    draw.rect(Vec2::ZERO, node.size())
        .color(Color::rgb(0.04, 0.07, 0.12));
    if waveform.samples.len() < 2 {
        return;
    }

    let last = (waveform.samples.len() - 1) as f32;
    for (index, pair) in waveform.samples.windows(2).enumerate() {
        let start = vec2(
            index as f32 / last * node.size().x,
            (0.5 - pair[0] * 0.4) * node.size().y,
        );
        let end = vec2(
            (index + 1) as f32 / last * node.size().x,
            (0.5 - pair[1] * 0.4) * node.size().y,
        );
        draw.line(start, end).width(2.0).color(waveform.color);
    }
}

fn measure_waveform(input: UIMeasureInput, waveform: &Waveform) -> Vec2 {
    if waveform.invalid_measure {
        return vec2(-1.0, f32::NAN);
    }

    vec2(
        input
            .known_width
            .unwrap_or(80.0 + waveform.samples.len() as f32 * 22.0),
        input.known_height.unwrap_or(82.0),
    )
}

fn update_layout(mut layout: ResMut<UILayout<MainLayout>>, window: Res<Window>) {
    layout.set_size(window.size());
}

fn controls(
    keyboard: Res<Keyboard>,
    chart: Single<(&mut Waveform, &mut UIMeasure), With<IntrinsicWaveform>>,
) {
    let (mut waveform, mut measure) = chart.into_inner();

    if keyboard.just_pressed(KeyCode::Digit1) {
        let count = if waveform.samples.len() == 10 { 22 } else { 10 };
        waveform.samples = samples(count);
        waveform.invalid_measure = false;
        measure.invalidate();
    }
    if keyboard.just_pressed(KeyCode::KeyI) {
        waveform.invalid_measure = true;
        measure.invalidate();
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        waveform.invalid_measure = false;
        measure.invalidate();
    }
}

fn count_layout_updates(mut updates: MessageReader<UILayoutUpdateEvent<MainLayout>>) {
    let count = updates.read().count();
    if count != 0 {
        println!("layout updates: {count}");
    }
}

fn record_invalid_measure(
    error: On<UIRuntimeError>,
    charts: Query<(), With<IntrinsicWaveform>>,
    mut evidence: ResMut<ChartEvidence>,
) {
    if let UIRuntimeError::InvalidMeasure { entity, .. } = &*error
        && charts.contains(*entity)
    {
        evidence.invalid_measures += 1;
        println!("invalid waveform measurement reported for {entity:?}; layout used zero size");
    }
}

fn show_status(
    evidence: Res<ChartEvidence>,
    fixed: Single<&UINode, With<FixedWaveform>>,
    intrinsic: Single<&UINode, With<IntrinsicWaveform>>,
    mut status: Single<&mut UIText, With<ChartStatus>>,
) {
    let text = format!(
        "Fixed: {:.0} x {:.0} (clipped)\nIntrinsic: {:.0} x {:.0}\nInvalid: {} | Draw: {} elements / {} batches",
        fixed.size().x,
        fixed.size().y,
        intrinsic.size().x,
        intrinsic.size().y,
        evidence.invalid_measures,
        evidence.draw.elements,
        evidence.draw.batches,
    );
    if status.text != text {
        status.text = text;
    }
}

fn draw(world: &mut World) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw_ui_layout::<MainLayout>(&mut draw, world);
    world.resource_mut::<ChartEvidence>().draw = draw.stats();
    gfx::render_to_frame(&draw).unwrap();
}
