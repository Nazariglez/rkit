use rkit::gfx::{
    self, BindGroup, BindGroupLayout, BindingType, BlendMode, Buffer, Color, IndexFormat,
    RenderPipeline, Renderer, TextureFormat, VertexFormat, VertexLayout,
};

// language=wgsl
const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position.x, model.position.y * -1.0, 0.0, 1.0);
    return out;
}

@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_texture: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_texture, s_texture, in.tex_coords);
}
"#;

struct State {
    pip: RenderPipeline,
    vbo: Buffer,
    ebo: Buffer,
    bind_group: BindGroup,
}

impl State {
    fn new() -> Result<Self, String> {
        let pip = gfx::create_render_pipeline(SHADER)
            .with_label("Draw2D shapes default pipeline")
            .with_vertex_layout(
                VertexLayout::new()
                    .with_attr(0, VertexFormat::Float32x2)
                    .with_attr(1, VertexFormat::Float32x2),
            )
            .with_bind_group_layout(
                BindGroupLayout::new()
                    .with_entry(BindingType::texture(0).with_fragment_visibility(true))
                    .with_entry(BindingType::sampler(1).with_fragment_visibility(true)),
            )
            .with_index_format(IndexFormat::UInt16)
            .with_blend_mode(BlendMode::NORMAL)
            .build()?;

        let texture = gfx::create_texture()
            .from_image(include_bytes!("assets/ferris.png"))
            .build()?;

        let sampler = gfx::create_sampler().build()?;

        let bind_group = gfx::create_bind_group()
            .with_layout(pip.bind_group_layout_ref(0)?)
            .with_texture(0, &texture)
            .with_sampler(1, &sampler)
            .build()?;

        #[rustfmt::skip]
        let vertices: &[f32] = &[
            //pos           //coords
            0.5,  0.5,     1.0, 1.0,
            0.5, -0.5,     1.0, 0.0,
            -0.5, -0.5,     0.0, 0.0,
            -0.5,  0.5,     0.0, 1.0,
        ];
        let vbo = gfx::create_vertex_buffer(vertices).build()?;

        #[rustfmt::skip]
        let indices: &[u16] = &[
            0, 1, 3,
            1, 2, 3,
        ];
        let ebo = gfx::create_index_buffer(indices).build()?;

        Ok(State {
            pip,
            vbo,
            ebo,
            bind_group,
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
    let mut renderer = Renderer::new();
    renderer
        .begin_pass()
        .clear_color(Color::rgb(0.1, 0.2, 0.3))
        .pipeline(&s.pip)
        .buffers(&[&s.vbo, &s.ebo])
        .bindings(&[&s.bind_group])
        .draw(0..6);

    gfx::render_to_frame(&renderer).unwrap();
}
