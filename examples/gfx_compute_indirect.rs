use rkit::gfx::{
    self, BindGroup, Buffer, ComputePipeline, DispatchArgs, DrawArgs, IndirectBuffer,
    RenderPipeline, Renderer,
};
use rkit::time;

const SEED: &str = r#"
@group(0) @binding(0) var<storage, read_write> dispatch_args: array<u32>;
@group(0) @binding(1) var<storage, read_write> draw_args: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read> candidates: array<f32>;
@compute @workgroup_size(1)
fn cs_main() {
    dispatch_args[0] = arrayLength(&candidates);
    dispatch_args[1] = 1u;
    dispatch_args[2] = 1u;
    atomicStore(&draw_args[0], 6u);
    atomicStore(&draw_args[1], 0u);
    atomicStore(&draw_args[2], 0u);
    atomicStore(&draw_args[3], 0u);
}
"#;

const SELECT: &str = r#"
@group(0) @binding(0) var<storage, read_write> draw_args: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read> candidates: array<f32>;
@group(0) @binding(2) var<storage, read> threshold: array<f32>;
@compute @workgroup_size(1)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    if (candidates[id.x] < threshold[0]) {
        atomicAdd(&draw_args[1], 1u);
    }
}
"#;

const RENDER: &str = r#"
struct Output { @builtin(position) position: vec4f, @location(0) color: vec3f };
@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> Output {
    let corners = array<vec2f, 6>(vec2f(-1.0f, -1.0f), vec2f(1.0f, -1.0f), vec2f(1.0f, 1.0f), vec2f(-1.0f, -1.0f), vec2f(1.0f, 1.0f), vec2f(-1.0f, 1.0f));
    let x = f32(instance % 12u) / 6.0f - 0.92f;
    let y = f32(instance / 12u) / 4.0f - 0.86f;
    var output: Output;
    output.position = vec4f(vec2f(x, y) + corners[vertex] * 0.055f, 0.0f, 1.0f);
    output.color = vec3f(fract(f32(instance) * 0.17f), 0.55f, 1.0f - fract(f32(instance) * 0.11f));
    return output;
}
@fragment fn fs_main(input: Output) -> @location(0) vec4f { return vec4f(input.color, 1.0f); }
"#;

struct State {
    seed: ComputePipeline,
    select: ComputePipeline,
    render: RenderPipeline,
    seed_group: BindGroup,
    select_group: BindGroup,
    dispatch: IndirectBuffer<DispatchArgs>,
    draw: IndirectBuffer<DrawArgs>,
    threshold: Buffer,
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
        .unwrap();
}

impl State {
    fn new() -> Result<Self, String> {
        let dispatch = gfx::create_indirect_buffer::<DispatchArgs>().build()?;
        let draw = gfx::create_indirect_buffer::<DrawArgs>().build()?;
        let seed = gfx::create_compute_pipeline(SEED).build()?;
        let select = gfx::create_compute_pipeline(SELECT).build()?;
        let candidates = gfx::create_storage_buffer(
            &(0..96)
                .map(|index| ((index * 37 % 97) as f32) / 97.0)
                .collect::<Vec<_>>(),
        )
        .build()?;
        let threshold = gfx::create_storage_buffer(&[0.0_f32])
            .with_write_flag(true)
            .build()?;
        let seed_group = gfx::create_bind_group()
            .with_layout(seed.bind_group_layout_ref(0)?)
            .with_storage_readwrite(0, &dispatch)
            .with_storage_readwrite(1, &draw)
            .with_storage_readonly(2, &candidates)
            .build()?;
        let select_group = gfx::create_bind_group()
            .with_layout(select.bind_group_layout_ref(0)?)
            .with_storage_readwrite(0, &draw)
            .with_storage_readonly(1, &candidates)
            .with_storage_readonly(2, &threshold)
            .build()?;
        Ok(Self {
            seed,
            select,
            render: gfx::create_render_pipeline(RENDER).build()?,
            seed_group,
            select_group,
            dispatch,
            draw,
            threshold,
        })
    }
}

fn update(state: &mut State) {
    let threshold = 0.05 + 0.9 * (time::elapsed_f32().sin() * 0.5 + 0.5);
    let mut compute = gfx::Compute::new();
    compute
        .write_buffer(&state.threshold)
        .with_data(&[threshold])
        .build()
        .unwrap();
    compute
        .dispatch_workgroups(&state.seed, [1, 1, 1])
        .bindings(&[&state.seed_group]);
    compute
        .dispatch_indirect(&state.select, &state.dispatch)
        .bindings(&[&state.select_group]);
    gfx::compute(&compute).unwrap();
    let mut renderer = Renderer::new();
    renderer
        .begin_pass()
        .clear_color(rkit::gfx::Color::rgb(0.02, 0.03, 0.07))
        .pipeline(&state.render)
        .draw_indirect(&state.draw);
    gfx::render_to_frame(&renderer).unwrap();
}
