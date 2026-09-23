use rkit::gfx::{self, BindGroup, Buffer, Color, ComputePipeline, RenderPipeline, Renderer};
use rkit::time;

const FIELD_SIZE: u32 = 256;

const COMPUTE: &str = r#"
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var output_field: texture_storage_2d<rg32float, write>;
@group(0) @binding(2) var<storage, read> params: vec4f;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    let size = textureDimensions(source);
    if (id.x >= size.x || id.y >= size.y) { return; }
    let uv = vec2f(id.xy) / vec2f(size);
    let previous = textureLoad(source, vec2i(id.xy), 0).xy;
    let wave = sin((uv.x + params.x * 0.14f) * 18.0f) * cos((uv.y - params.x * 0.11f) * 15.0f);
    let next = mix(previous, vec2f(0.5f + 0.5f * wave, 0.5f - 0.5f * wave), 0.08f);
    textureStore(output_field, vec2i(id.xy), vec4f(next, 0.0f, 1.0f));
}
"#;

const RENDER: &str = r#"
@group(0) @binding(0) var field: texture_2d<f32>;

struct Output {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32) -> Output {
    let positions = array<vec2f, 3>(vec2f(-1., -1.), vec2f(3., -1.), vec2f(-1., 3.));
    var output: Output;
    output.position = vec4f(positions[vertex], 0., 1.);
    output.uv = positions[vertex] * 0.5 + 0.5;
    return output;
}

@fragment
fn fs_main(input: Output) -> @location(0) vec4f {
    let size = textureDimensions(field);
    let pixel = vec2i(clamp(input.uv, vec2f(0.), vec2f(0.999)) * vec2f(size));
    let value = textureLoad(field, pixel, 0).xy;
    return vec4f(value.x, value.y, 1. - value.x * 0.7, 1.);
}
"#;

struct State {
    compute: ComputePipeline,
    render: RenderPipeline,
    params: Buffer,
    forward: BindGroup,
    backward: BindGroup,
    ping_view: BindGroup,
    pong_view: BindGroup,
    ping_is_source: bool,
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
        .unwrap();
}

impl State {
    fn new() -> Result<Self, String> {
        let ping = gfx::create_storage_texture()
            .with_size(FIELD_SIZE, FIELD_SIZE)
            .with_format(gfx::TextureFormat::Rg32Float)
            .build()?;
        let pong = gfx::create_storage_texture()
            .with_size(FIELD_SIZE, FIELD_SIZE)
            .with_format(gfx::TextureFormat::Rg32Float)
            .build()?;
        let params = gfx::create_storage_buffer(&[[0.0_f32; 4]])
            .with_write_flag(true)
            .build()?;
        let compute = gfx::create_compute_pipeline(COMPUTE).build()?;
        let render = gfx::create_render_pipeline(RENDER).build()?;
        let forward = gfx::create_bind_group()
            .with_layout(compute.bind_group_layout_ref(0)?)
            .with_texture(0, &ping)
            .with_storage_texture_writeonly(1, &pong)
            .with_storage_readonly(2, &params)
            .build()?;
        let backward = gfx::create_bind_group()
            .with_layout(compute.bind_group_layout_ref(0)?)
            .with_texture(0, &pong)
            .with_storage_texture_writeonly(1, &ping)
            .with_storage_readonly(2, &params)
            .build()?;
        let ping_view = gfx::create_bind_group()
            .with_layout(render.bind_group_layout_ref(0)?)
            .with_texture(0, &ping)
            .build()?;
        let pong_view = gfx::create_bind_group()
            .with_layout(render.bind_group_layout_ref(0)?)
            .with_texture(0, &pong)
            .build()?;
        Ok(Self {
            compute,
            render,
            params,
            forward,
            backward,
            ping_view,
            pong_view,
            ping_is_source: true,
        })
    }
}

fn update(state: &mut State) {
    let mut compute = gfx::Compute::new();
    let mut ping_is_source = state.ping_is_source;
    for step in 0..4 {
        compute
            .write_buffer(&state.params)
            .with_data(&[[
                time::elapsed_f32() + step as f32 * 0.08,
                step as f32,
                0.0,
                0.0,
            ]])
            .build()
            .unwrap();
        let source = if ping_is_source {
            &state.forward
        } else {
            &state.backward
        };
        compute
            .dispatch_for(&state.compute, [FIELD_SIZE, FIELD_SIZE, 1])
            .bindings(&[source]);
        ping_is_source = !ping_is_source;
    }
    gfx::compute(&compute).unwrap();
    state.ping_is_source = ping_is_source;

    let field = if state.ping_is_source {
        &state.ping_view
    } else {
        &state.pong_view
    };
    let mut renderer = Renderer::new();
    renderer
        .begin_pass()
        .clear_color(Color::BLACK)
        .pipeline(&state.render)
        .bindings(&[field])
        .draw(0..3);
    gfx::render_to_frame(&renderer).unwrap();
}
