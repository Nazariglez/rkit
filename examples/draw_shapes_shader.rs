use draw::{AsBindGroups, DrawPipelineId, PipelineContext, PipelineResources};
use rkit::draw::create_draw_2d;
use rkit::gfx::{self, BindGroupLayout, BindingType, Buffer, Color, VertexFormat, VertexLayout};
use rkit::math::vec2;
use rkit::time;

// language=wgsl
const SHADER: &str = r#"
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
    @location(1) frag_pos: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.frag_pos = model.position;
    out.position = transform.mvp * vec4(model.position, 0.0, 1.0);
    return out;
}

struct Effect {
    pos: vec2<f32>,
    size: vec2<f32>,
    time: f32,
    _padding: f32,
};

@group(1) @binding(0)
var<uniform> effect: Effect;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos_relative = in.frag_pos - effect.pos;
    let pos_ndc = 2.0 * (pos_relative / effect.size);
    let dist = length(pos_ndc);

    let c1 = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    let c2 = vec4<f32>(effect.time, 1.0-effect.time, 0.0, 1.0);
    let c3 = vec4<f32>(1.0-effect.time, 0.0, effect.time, 1.0);
    let c4 = vec4<f32>(0.0, effect.time, 1.0-effect.time, 1.0);

    let step1 = fract(0.0 + effect.time);
    let step2 = fract(0.25 + effect.time);
    let step3 = fract(0.75 + effect.time);
    let step4 = fract(1.0 + effect.time);

    var color = mix(c1, c2, smoothstep(step1, step2, dist));
    color = mix(color, c3, smoothstep(step2, step3, dist));
    color = mix(color, c4, smoothstep(step3, step4, dist));

    return color;
}
"#;

pub fn custom_pipeline(
    res: PipelineResources,
    ubo_effect: &Buffer,
) -> Result<PipelineContext, String> {
    let pip = gfx::create_render_pipeline(SHADER)
        .with_vertex_layout(
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x4),
        )
        .with_bind_group_layout(
            BindGroupLayout::new().with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
        )
        // bind the UBO that contains the effect data
        .with_bind_group_layout(
            BindGroupLayout::new()
                .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
        )
        .build()?;

    let transform_bg = gfx::create_bind_group()
        .with_layout(pip.bind_group_layout_ref(0)?)
        .with_uniform(0, res.ubo)
        .build()?;

    let effect_bg = gfx::create_bind_group()
        .with_layout(pip.bind_group_layout_ref(1)?)
        .with_uniform(0, ubo_effect)
        .build()?;

    Ok(PipelineContext {
        pipeline: pip,
        groups: (&[transform_bg, effect_bg]).as_bind_groups(),
        vertex_offset: 6,
        x_pos: 0,
        y_pos: 1,
        alpha_pos: Some(5),
    })
}

struct State {
    ubo: Buffer,
    pip_id: DrawPipelineId,
}

impl State {
    fn new() -> Self {
        let ubo = gfx::create_uniform_buffer(&[0.0f32; 6])
            .with_write_flag(true)
            .build()
            .unwrap();

        let pip_id = draw::add_pipeline_2d(|res| custom_pipeline(res, &ubo).unwrap());

        Self { ubo, pip_id }
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(State::new).update(update).run()
}

fn update(s: &mut State) {
    let t = time::elapsed_f32().cos() / 2.0;
    let pos = vec2(600.0 + t * 50.0, 300.0);
    let size = vec2(400.0, 300.0);
    gfx::write_buffer(&s.ubo)
        .with_data(&[pos.x, pos.y, size.x, size.y, t, 0.0])
        .build()
        .unwrap();

    let mut draw = create_draw_2d();
    draw.clear(Color::BLACK);

    // star without custom pipeline
    draw.star(5, 150.0, 70.0).position(vec2(200.0, 300.0));

    draw.star(5, 150.0, 70.0)
        .position(pos)
        .color(Color::GRAY)
        .pipeline(&s.pip_id);

    gfx::render_to_frame(&draw).unwrap();
}
