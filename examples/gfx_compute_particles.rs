use rkit::gfx::{self, BindGroup, Buffer, Color, ComputePipeline, RenderPipeline, Renderer};
use rkit::time;

const PARTICLES: usize = 256;

const COMPUTE: &str = r#"
struct Particle {
    position: vec2f,
    velocity: vec2f,
    color: vec4f,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<storage, read> params: vec4f;

@compute @workgroup_size(64)
fn emit(@builtin(global_invocation_id) id: vec3u) {
    if (id.x >= arrayLength(&particles)) { return; }
    var particle = particles[id.x];
    if (particle.position.y > 1.15f) {
        let seed = f32(id.x) * 0.618f;
        particle.position = vec2f(fract(seed) * 2.0f - 1.0f, -1.1f);
        particle.velocity = vec2f(sin(seed * 19.0f) * 0.12f, 0.18f + fract(seed) * 0.25f);
    }
    particles[id.x] = particle;
}

@compute @workgroup_size(64)
fn update(@builtin(global_invocation_id) id: vec3u) {
    if (id.x >= arrayLength(&particles)) { return; }
    var particle = particles[id.x];
    particle.position += particle.velocity * params.y;
    particles[id.x] = particle;
}
"#;

const RENDER: &str = r#"
struct Particle {
    position: vec2f,
    velocity: vec2f,
    color: vec4f,
};

@group(0) @binding(0) var<storage, read> particles: array<Particle>;

struct Output {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> Output {
    let corners = array<vec2f, 6>(
        vec2f(-1., -1.), vec2f(1., -1.), vec2f(1., 1.),
        vec2f(-1., -1.), vec2f(1., 1.), vec2f(-1., 1.),
    );
    let particle = particles[instance];
    var output: Output;
    output.position = vec4f(particle.position + corners[vertex] * 0.018, 0., 1.);
    output.color = particle.color;
    return output;
}

@fragment
fn fs_main(input: Output) -> @location(0) vec4f { return input.color; }
"#;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    position: [f32; 2],
    velocity: [f32; 2],
    color: [f32; 4],
}

struct State {
    emit: ComputePipeline,
    update: ComputePipeline,
    render: RenderPipeline,
    params: Buffer,
    emit_group: BindGroup,
    update_group: BindGroup,
    render_group: BindGroup,
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
        .unwrap();
}

impl State {
    fn new() -> Result<Self, String> {
        let particles = (0..PARTICLES)
            .map(|index| {
                let seed = index as f32 / PARTICLES as f32;
                Particle {
                    position: [seed * 2.0 - 1.0, seed * 2.0 - 1.0],
                    velocity: [0.12 * (seed * 19.0).sin(), 0.18 + seed * 0.25],
                    color: [0.25 + seed * 0.75, 0.4, 1.0 - seed * 0.5, 1.0],
                }
            })
            .collect::<Vec<_>>();
        let particles = gfx::create_storage_buffer(&particles).build()?;
        let params = gfx::create_storage_buffer(&[[0.0_f32; 4]])
            .with_write_flag(true)
            .build()?;
        let shader = gfx::create_shader(COMPUTE).build()?;
        let emit = gfx::create_compute_pipeline(&shader)
            .with_entry("emit")
            .build()?;
        let update = gfx::create_compute_pipeline(&shader)
            .with_entry("update")
            .build()?;
        let render = gfx::create_render_pipeline(RENDER).build()?;
        let emit_group = gfx::create_bind_group()
            .with_layout(emit.bind_group_layout_ref(0)?)
            .with_storage_readwrite(0, &particles)
            .build()?;
        let update_group = gfx::create_bind_group()
            .with_layout(update.bind_group_layout_ref(0)?)
            .with_storage_readwrite(0, &particles)
            .with_storage_readonly(1, &params)
            .build()?;
        let render_group = gfx::create_bind_group()
            .with_layout(render.bind_group_layout_ref(0)?)
            .with_storage_readonly(0, &particles)
            .build()?;
        Ok(Self {
            emit,
            update,
            render,
            params,
            emit_group,
            update_group,
            render_group,
        })
    }
}

fn update(state: &mut State) {
    let mut compute = gfx::Compute::new();
    compute
        .write_buffer(&state.params)
        .with_data(&[[time::elapsed_f32(), time::delta_f32(), 0.0, 0.0]])
        .build()
        .unwrap();
    compute
        .dispatch_for(&state.emit, [PARTICLES as u32, 1, 1])
        .bindings(&[&state.emit_group]);
    compute
        .dispatch_for(&state.update, [PARTICLES as u32, 1, 1])
        .bindings(&[&state.update_group]);
    gfx::compute(&compute).unwrap();

    let mut renderer = Renderer::new();
    renderer
        .begin_pass()
        .clear_color(Color::rgb(0.015, 0.025, 0.06))
        .pipeline(&state.render)
        .bindings(&[&state.render_group])
        .draw_instanced(0..6, PARTICLES as u32);
    gfx::render_to_frame(&renderer).unwrap();
}
