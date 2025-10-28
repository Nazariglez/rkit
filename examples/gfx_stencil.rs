use rkit::gfx::{
    self, Buffer, Color, CompareMode, RenderPipeline, Renderer, Stencil, StencilAction,
    VertexFormat, VertexLayout,
};

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

struct State {
    mask_pip: RenderPipeline,
    mask_vbo: Buffer,
    pip: RenderPipeline,
    vbo: Buffer,
}

impl State {
    fn new() -> Result<Self, String> {
        let mask_pip = gfx::create_render_pipeline(SHADER)
            .with_label("Mask Pipeline")
            .with_vertex_layout(
                VertexLayout::new()
                    .with_attr(0, VertexFormat::Float32x2)
                    .with_attr(1, VertexFormat::Float32x3),
            )
            .with_stencil(Stencil {
                stencil_fail: StencilAction::Keep,
                depth_fail: StencilAction::Keep,
                pass: StencilAction::Replace,
                compare: CompareMode::Always,
                read_mask: 0xff,
                write_mask: 0xff,
                reference: 1,
            })
            .build()?;

        let pip = gfx::create_render_pipeline(SHADER)
            .with_label("Main Pipeline")
            .with_vertex_layout(
                VertexLayout::new()
                    .with_attr(0, VertexFormat::Float32x2)
                    .with_attr(1, VertexFormat::Float32x3),
            )
            .with_stencil(Stencil {
                stencil_fail: StencilAction::Keep,
                depth_fail: StencilAction::Keep,
                pass: StencilAction::Keep,
                compare: CompareMode::Equal,
                read_mask: 0xff,
                write_mask: 0x00,
                reference: 1,
            })
            .build()?;

        #[rustfmt::skip]
        let mask_vertices: &[f32] = &[
            0.5, 1.35, 1.0, 1.0, 1.0,
            0.25, 0.85, 1.0, 1.0, 1.0,
            0.75, 0.85, 1.0, 1.0, 1.0,

            0.75, 0.85, 1.0, 1.0, 1.0,
            0.5, 0.35, 1.0, 1.0, 1.0,
            1.0, 0.35, 1.0, 1.0, 1.0,

            0.25, 0.85, 1.0, 1.0, 1.0,
            0.0, 0.35, 1.0, 1.0, 1.0,
            0.5, 0.35, 1.0, 1.0, 1.0,

            0.5, 0.35, 1.0, 1.0, 1.0,
            0.25, -0.15, 1.0, 1.0, 1.0,
            0.75, -0.15, 1.0, 1.0, 1.0,

            1.0, 0.35, 1.0, 1.0, 1.0,
            0.75, -0.15, 1.0, 1.0, 1.0,
            1.25, -0.15, 1.0, 1.0, 1.0,

            0.0, 0.35, 1.0, 1.0, 1.0,
            -0.25, -0.15, 1.0, 1.0, 1.0,
            0.25, -0.15, 1.0, 1.0, 1.0,
        ];

        let mask_vbo = gfx::create_vertex_buffer(mask_vertices).build()?;

        #[rustfmt::skip]
        let vertices: &[f32] = &[
            0.5, 1.0, 1.0, 0.2, 0.3,
            0.0, 0.0, 0.1, 1.0, 0.3,
            1.0, 0.0, 0.1, 0.2, 1.0,
        ];

        let vbo = gfx::create_vertex_buffer(vertices).build()?;

        Ok(State {
            pip,
            vbo,
            mask_pip,
            mask_vbo,
        })
    }
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
        .unwrap()
}

fn update(s: &mut State) {
    let mut renderer = Renderer::new();
    renderer
        .begin_pass()
        .clear_color(Color::rgb(0.1, 0.2, 0.3))
        .clear_stencil(0)
        .pipeline(&s.mask_pip)
        .buffers(&[&s.mask_vbo])
        .stencil_reference(1)
        .draw(0..18);

    // TODO this must be same pass
    renderer
        .begin_pass()
        .pipeline(&s.pip)
        .buffers(&[&s.vbo])
        .stencil_reference(1)
        .draw(0..3);

    gfx::render_to_frame(&renderer).unwrap();
}
