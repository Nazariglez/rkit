use rkit::gfx::{self, Buffer, Color, RenderPipeline, Renderer, VertexFormat, VertexLayout};

// language=wgsl
const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.position = vec4<f32>(model.position - 0.5, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

struct AppState {
    pip: RenderPipeline,
    vbo: Buffer,
}

fn main() {
    rkit::init_with(AppState::new)
        .on_update(update)
        .run()
        .unwrap()
}

impl AppState {
    fn new() -> Self {
        let pip = gfx::create_render_pipeline(SHADER)
            .with_vertex_layout(
                VertexLayout::new()
                    .with_attr(0, VertexFormat::Float32x2)
                    .with_attr(1, VertexFormat::Float32x3),
            )
            .build()
            .unwrap();

        #[rustfmt::skip]
        let vertices: &[f32] = &[
            0.5, 1.0,   1.0, 0.0, 0.0,
            0.0, 0.0,   0.0, 1.0, 0.0,
            1.0, 0.0,   0.0, 0.0, 1.0,
        ];

        let vbo = gfx::create_vertex_buffer(vertices).build().unwrap();
        AppState { pip, vbo }
    }
}

fn update(s: &mut AppState) {
    let mut renderer = Renderer::new();
    renderer
        .begin_pass()
        .clear_color(Color::rgb(0.1, 0.2, 0.3))
        .pipeline(&s.pip)
        .buffers(&[&s.vbo])
        .draw(0..3);

    gfx::render_to_frame(&renderer).unwrap();
}
