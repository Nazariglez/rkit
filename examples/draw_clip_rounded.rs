use draw::{AsBindGroups, DrawPipelineId, PipelineContext, PipelineResources};
use rkit::{
    draw::create_draw_2d,
    gfx::{self, BindGroupLayout, BindingType, Color, RenderTexture, VertexFormat, VertexLayout},
    input::{KeyCode, is_key_down, is_key_pressed},
    math::{Rect, Vec2, vec2},
    prelude::*,
};

const CUSTOM_SHADER: &str = r#"
struct Transform {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> transform: Transform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = transform.mvp * vec4(model.position, 0.0, 1.0);
    out.color = vec4(model.color.b, model.color.r, model.color.g, model.color.a);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

fn create_custom_pipeline(resources: PipelineResources) -> PipelineContext {
    let pipeline = gfx::create_render_pipeline(CUSTOM_SHADER)
        .with_label("Rounded clip custom Draw2D pipeline")
        .with_vertex_layout(
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x4),
        )
        .with_bind_group_layout(
            BindGroupLayout::new().with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
        )
        .build()
        .unwrap();
    let transform = gfx::create_bind_group()
        .with_layout(pipeline.bind_group_layout_ref(0).unwrap())
        .with_uniform(0, resources.ubo)
        .build()
        .unwrap();
    PipelineContext {
        pipeline,
        groups: (&[transform] as &[_]).to_bind_groups(),
        vertex_offset: 6,
        x_pos: 0,
        y_pos: 1,
        alpha_pos: Some(5),
    }
}

#[derive(Resource)]
struct State {
    offset: Vec2,
    radius: f32,
    nested: bool,
    unclosed: bool,
    no_depth: bool,
    error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            radius: 64.0,
            nested: true,
            unclosed: false,
            no_depth: false,
            error: None,
        }
    }
}

#[derive(Resource)]
struct CustomPipeline(DrawPipelineId);

#[derive(Resource)]
struct Targets {
    with_depth: RenderTexture,
    without_depth: RenderTexture,
}

fn main() -> Result<(), String> {
    App::new()
        .add_plugin(MainPlugins::default())
        .insert_resource(State::default())
        .on_setup(setup)
        .on_update(update)
        .on_render(render)
        .run()
}

fn setup(mut commands: Commands) {
    let custom = draw::add_pipeline_2d(create_custom_pipeline);
    let with_depth = gfx::create_render_texture()
        .with_label("Rounded clip depth target")
        .with_size(400, 260)
        .with_depth(true)
        .build()
        .unwrap();
    let without_depth = gfx::create_render_texture()
        .with_label("Rounded clip no-depth target")
        .with_size(400, 260)
        .build()
        .unwrap();
    commands.insert_resource(Targets {
        with_depth,
        without_depth,
    });
    commands.insert_resource(CustomPipeline(custom));
}

fn update(mut state: ResMut<State>, time: Res<Time>) {
    let movement = time.delta_f32() * 180.0;
    if is_key_down(KeyCode::KeyA) {
        state.offset.x -= movement;
    }
    if is_key_down(KeyCode::KeyD) {
        state.offset.x += movement;
    }
    if is_key_down(KeyCode::KeyW) {
        state.offset.y -= movement;
    }
    if is_key_down(KeyCode::KeyS) {
        state.offset.y += movement;
    }
    if is_key_down(KeyCode::KeyQ) {
        state.radius -= movement * 0.5;
    }
    if is_key_down(KeyCode::KeyE) {
        state.radius += movement * 0.5;
    }
    if is_key_pressed(KeyCode::KeyN) {
        state.nested = !state.nested;
    }
    if is_key_pressed(KeyCode::KeyU) {
        state.unclosed = !state.unclosed;
    }
    if is_key_pressed(KeyCode::KeyT) {
        state.no_depth = !state.no_depth;
    }
}

fn render(mut state: ResMut<State>, targets: Res<Targets>, custom: Res<CustomPipeline>) {
    let mut draw = create_draw_2d();
    draw.clear(Color::rgb(0.04, 0.05, 0.08));
    draw.rect(vec2(30.0, 35.0), vec2(80.0, 40.0))
        .color(Color::rgb(0.2, 0.75, 0.45));
    draw.push_rounded_clip(
        Rect::new(vec2(140.0, 100.0), vec2(520.0, 350.0)),
        state.radius,
    );
    draw.rect(vec2(55.0, 35.0) + state.offset, vec2(700.0, 480.0))
        .color(Color::rgb(0.12, 0.35, 0.65));
    draw.text("Move this text through every curved corner")
        .position(vec2(85.0, 245.0) + state.offset)
        .size(30.0)
        .resolution(1.0);
    draw.rect(vec2(205.0, 125.0) + state.offset, vec2(390.0, 70.0))
        .color(Color::rgb(0.7, 0.2, 0.8))
        .pipeline(&custom.0);

    if state.nested {
        draw.push_rounded_clip(Rect::new(vec2(270.0, 190.0), vec2(260.0, 170.0)), 42.0);
        draw.circle(145.0)
            .position(vec2(400.0, 275.0) + state.offset)
            .color(Color::ORANGE);
        draw.pop_clip();
        draw.push_clip(Rect::new(vec2(185.0, 145.0), vec2(430.0, 250.0)));
        draw.rect(vec2(100.0, 320.0) + state.offset, vec2(600.0, 95.0))
            .color(Color::rgb(0.75, 0.18, 0.32));
        draw.pop_clip();
    }
    if !state.unclosed {
        draw.pop_clip();
        draw.rect(vec2(690.0, 390.0), vec2(80.0, 40.0))
            .color(Color::rgb(0.2, 0.75, 0.45));
    }

    let target = if state.no_depth {
        &targets.without_depth
    } else {
        &targets.with_depth
    };
    state.error = gfx::render_to_texture(target, &draw).err();
    if let Err(error) = gfx::render_to_frame(&draw) {
        state.error = Some(error);
    }

    let mut overlay = create_draw_2d();
    overlay
        .text("WASD move | Q/E radius | N nested | U unclosed | T target depth")
        .position(vec2(35.0, 515.0))
        .size(19.0);
    overlay
        .text(&format!(
            "radius: {:.0} | nested: {} | unclosed: {} | no depth: {}",
            state.radius, state.nested, state.unclosed, state.no_depth
        ))
        .position(vec2(35.0, 545.0))
        .size(18.0);
    if let Some(error) = &state.error {
        overlay.text(error).position(vec2(35.0, 575.0)).size(16.0);
    }
    gfx::render_to_frame(&overlay).unwrap();
}
