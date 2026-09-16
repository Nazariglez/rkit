use rkit::gfx::{
    self, BindGroup, BindGroupLayout, BindingType, Buffer, Color, RenderPipeline, Renderer,
    VertexFormat, VertexLayout,
};
use rkit::time;

const SHADER: &str = r#"
@group(0) @binding(0)
var<storage, read> colors: array<vec4<f32>>;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) color_index: u32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let color_index = input.instance_index % 2u;
    let color = colors[color_index];
    let offset = f32(input.instance_index) * 0.8 - 0.4;
    var output: VertexOutput;
    output.position = vec4<f32>(input.position + vec2<f32>(offset, (color.r - color.b) * 0.08), 0.0, 1.0);
    output.color_index = color_index;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let next = (input.color_index + 1u) % 2u;
    return mix(colors[input.color_index], colors[next], 0.3);
}
"#;

struct State {
    pipeline: RenderPipeline,
    vertices: Buffer,
    storage: Buffer,
    bind_group: BindGroup,
    replaced: bool,
}

impl State {
    fn new() -> Result<Self, String> {
        let limits = gfx::limits();
        log::info!(
            "Storage limits: allocation={} binding={} stage_bindings={}",
            limits.max_buffer_size,
            limits.max_storage_binding_size,
            limits.max_storage_buffers_per_shader_stage,
        );

        let pipeline = gfx::create_render_pipeline(SHADER)
            .with_vertex_layout(VertexLayout::new().with_attr(0, VertexFormat::Float32x2))
            .with_bind_group_layout(
                BindGroupLayout::new().with_entry(
                    BindingType::storage_readonly(0)
                        .with_vertex_visibility(true)
                        .with_fragment_visibility(true),
                ),
            )
            .build()?;
        let vertices: [[f32; 2]; 3] = [[-0.25, -0.25], [0.25, -0.25], [0.0, 0.25]];
        let storage_colors: [[f32; 4]; 2] = [[0.7, 0.2, 0.9, 1.0], [0.2, 0.8, 0.5, 1.0]];
        let vertices = gfx::create_vertex_buffer(&vertices).build()?;
        let storage = gfx::create_storage_buffer(&storage_colors)
            .with_write_flag(true)
            .build()?;
        let bind_group = Self::bind(&pipeline, &storage)?;

        Ok(Self {
            pipeline,
            vertices,
            storage,
            bind_group,
            replaced: false,
        })
    }

    fn bind(pipeline: &RenderPipeline, storage: &Buffer) -> Result<BindGroup, String> {
        gfx::create_bind_group()
            .with_layout(pipeline.bind_group_layout_ref(0)?)
            .with_storage_readonly(0, storage)
            .build()
    }

    fn replace_storage(&mut self) -> Result<(), String> {
        let colors: [[f32; 4]; 3] = [
            [0.95, 0.65, 0.2, 1.0],
            [0.15, 0.45, 0.95, 1.0],
            [0.3, 0.95, 0.7, 1.0],
        ];
        let storage = gfx::create_storage_buffer(&colors)
            .with_write_flag(true)
            .build()?;
        let bind_group = Self::bind(&self.pipeline, &storage)?;
        self.storage = storage;
        self.bind_group = bind_group;
        Ok(())
    }
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
        .unwrap();
}

fn update(state: &mut State) {
    let pulse = (time::elapsed_f32().sin() + 1.0) * 0.5;
    gfx::write_buffer(&state.storage)
        .with_data(&[
            [0.3 + 0.6 * pulse, 0.2, 0.9 - 0.5 * pulse, 1.0],
            [0.2, 0.4 + 0.5 * pulse, 0.9, 1.0],
        ])
        .build()
        .unwrap();

    if !state.replaced && time::elapsed_f32() > 3.0 {
        state.replace_storage().unwrap();
        state.replaced = true;
    }

    let mut renderer = Renderer::new();
    renderer
        .begin_pass()
        .clear_color(Color::rgb(0.08, 0.1, 0.14))
        .pipeline(&state.pipeline)
        .buffers(&[&state.vertices])
        .bindings(&[&state.bind_group])
        .draw_instanced(0..3, 2);
    gfx::render_to_frame(&renderer).unwrap();
}
