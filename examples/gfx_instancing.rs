use rkit::gfx::{
    self, BindGroup, BindGroupLayout, BindingType, Buffer, Color, RenderPipeline, Renderer,
    VertexFormat, VertexLayout,
};
use rkit::time;

// Number of triangles to draw
const INSTANCES: usize = 1000;

// language=wgsl
const SHADER: &str = r#"
struct Locals {
    count: f32,
};

@group(0) @binding(0)
var<uniform> locals: Locals;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    // Values to change position and color
    let n = f32(model.instance_index) * 0.1;
    let j = f32(model.vertex_index) * 0.2;
    let pos = model.position - vec2<f32>(sin(n + locals.count), cos(n + locals.count)) * fract(n) * 0.9;

    var output: VertexOutput;
    output.color = vec3<f32>(fract(n - j), 1.0 - fract(n), fract(n + j));
    output.position = vec4<f32>(pos, 0.0, 1.0);

    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
};
"#;

struct State {
    pip: RenderPipeline,
    vbo: Buffer,
    ubo: Buffer,
    bind_group: BindGroup,
    count: f32,
}

impl State {
    fn new() -> Result<Self, String> {
        #[rustfmt::skip]
        let position: &[f32] = &[
            -0.2, -0.2,
            0.2, -0.2,
            0.0, 0.2
        ];

        let pip = gfx::create_render_pipeline(SHADER)
            .with_vertex_layout(VertexLayout::new().with_attr(0, VertexFormat::Float32x2))
            .with_bind_group_layout(
                BindGroupLayout::new()
                    .with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
            )
            .build()?;

        let vbo = gfx::create_vertex_buffer(position).build()?;

        let count: f32 = 0.0;
        let ubo = gfx::create_uniform_buffer(&[count])
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_layout(pip.bind_group_layout_ref(0)?)
            .with_uniform(0, &ubo)
            .build()?;

        Ok(State {
            pip,
            vbo,
            ubo,
            bind_group,
            count,
        })
    }
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .on_update(update)
        .run()
        .unwrap()
}

fn update(s: &mut State) {
    s.count += 0.15 * time::delta_f32();

    let mut renderer = Renderer::new();
    renderer
        .begin_pass()
        .clear_color(Color::rgb(0.1, 0.2, 0.3))
        .pipeline(&s.pip)
        .buffers(&[&s.vbo])
        .bindings(&[&s.bind_group])
        .draw_instanced(0..3, INSTANCES as _);

    gfx::render_to_frame(&renderer).unwrap();

    // update the uniform to animate the triangles
    gfx::write_buffer(&s.ubo)
        .with_data(&[s.count])
        .build()
        .unwrap();
}
