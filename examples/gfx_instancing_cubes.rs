use rkit::gfx::{
    self, BindGroup, BindGroupLayout, BindingType, Buffer, Color, CullMode, IndexFormat,
    RenderPipeline, Renderer, VertexFormat, VertexLayout, VertexStepMode,
};
use rkit::math::{Mat4, vec3};
use rkit::random;
use rkit::time;

const INSTANCES: u32 = 120;

// language=wgsl
const SHADER: &str = r#"
struct Transform {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> transform: Transform;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    let i = f32(model.instance_index);
    let n = select(i, 0.0, i % 2.0 == 0.0);
    let j = select(i, 0.0, i % 3.0 == 0.0);
    let pos = vec4<f32>(model.position.x - i * 2.0, model.position.y - n * 2.0, model.position.z - j * 2.0, 1.0);

    var out: VertexOutput;
    out.color = model.color;
    out.position = transform.mvp * pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

struct State {
    pip: RenderPipeline,
    pos_vbo: Buffer,
    color_vbo: Buffer,
    ebo: Buffer,
    ubo: Buffer,
    bind_group: BindGroup,
    angle: f32,
    mvp: Mat4,
}

impl State {
    fn new() -> Result<Self, String> {
        let pip = gfx::create_render_pipeline(SHADER)
            .with_vertex_layout(VertexLayout::new().with_attr(0, VertexFormat::Float32x3))
            .with_vertex_layout(
                VertexLayout::new()
                    .with_step_mode(VertexStepMode::Instance)
                    .with_attr(1, VertexFormat::Float32x4),
            )
            .with_bind_group_layout(
                BindGroupLayout::new()
                    .with_entry(BindingType::uniform(0).with_vertex_visibility(true)),
            )
            .with_index_format(IndexFormat::UInt16)
            .with_cull_mode(CullMode::Front)
            .build()?;

        #[rustfmt::skip]
        let vertices: &[f32] = &[
            -1.0, -1.0, -1.0,
            1.0, -1.0, -1.0,
            1.0,  1.0, -1.0,
            -1.0,  1.0, -1.0,

            -1.0, -1.0,  1.0,
            1.0, -1.0,  1.0,
            1.0,  1.0,  1.0,
            -1.0,  1.0,  1.0,

            -1.0, -1.0, -1.0,
            -1.0,  1.0, -1.0,
            -1.0,  1.0,  1.0,
            -1.0, -1.0,  1.0,

            1.0, -1.0, -1.0,
            1.0,  1.0, -1.0,
            1.0,  1.0,  1.0,
            1.0, -1.0,  1.0,

            -1.0, -1.0, -1.0,
            -1.0, -1.0,  1.0,
            1.0, -1.0,  1.0,
            1.0, -1.0, -1.0,

            -1.0,  1.0, -1.0,
            -1.0,  1.0,  1.0,
            1.0,  1.0,  1.0,
            1.0,  1.0, -1.0,
        ];
        let pos_vbo = gfx::create_vertex_buffer(vertices).build()?;

        let colors = (0..INSTANCES)
            .map(|_| Color::rgb(random::r#gen(), random::r#gen(), random::r#gen()).to_rgba())
            .collect::<Vec<_>>();

        let color_vbo = gfx::create_vertex_buffer(&colors).build()?;

        #[rustfmt::skip]
        let indices: &[u16] = &[
            0, 1, 2,  0, 2, 3,
            6, 5, 4,  7, 6, 4,
            8, 9, 10,  8, 10, 11,
            14, 13, 12,  15, 14, 12,
            16, 17, 18,  16, 18, 19,
            22, 21, 20,  23, 22, 20
        ];
        let ebo = gfx::create_index_buffer(indices).build()?;

        let mvp = create_mvp();
        let ubo = gfx::create_uniform_buffer(mvp.as_ref())
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_layout(pip.bind_group_layout_ref(0)?)
            .with_uniform(0, &ubo)
            .build()?;

        Ok(State {
            pip,
            pos_vbo,
            color_vbo,
            ebo,
            ubo,
            bind_group,
            angle: 0.0,
            mvp,
        })
    }

    fn rotated_mvp(&self) -> Mat4 {
        self.mvp * Mat4::from_rotation_x(self.angle) * Mat4::from_rotation_y(self.angle)
    }
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
        .unwrap()
}

fn update(s: &mut State) {
    s.angle += 0.6 * time::delta_f32();

    // update mvp
    gfx::write_buffer(&s.ubo)
        .with_data(s.rotated_mvp().as_ref())
        .build()
        .unwrap();

    let mut renderer = Renderer::new();
    renderer
        .begin_pass()
        .clear_color(Color::rgb(0.1, 0.2, 0.3))
        .clear_depth(0.0)
        .pipeline(&s.pip)
        .buffers(&[&s.pos_vbo, &s.color_vbo, &s.ebo])
        .bindings(&[&s.bind_group])
        .draw_instanced(0..36, INSTANCES as _);

    gfx::render_to_frame(&renderer).unwrap();
}

fn create_mvp() -> Mat4 {
    let projection = Mat4::perspective_rh_gl(45.0, 4.0 / 3.0, 0.1, 100.0);
    let view = Mat4::look_at_rh(
        vec3(4.0, 3.0, 3.0),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
    );
    Mat4::IDENTITY * projection * view
}
