use rkit::gfx::{
    self, BindGroup, BindGroupLayout, BindingType, BlendMode, Buffer, Color, IndexFormat,
    RenderPipeline, RenderTexture, Renderer, TextureFormat, VertexFormat, VertexLayout,
};
use std::ops::Range;

const RENDER_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UNormSrgb;
const GENERATED_LEVELS: u32 = 9;

// language=wgsl
const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
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
    render_pipeline: RenderPipeline,
    display_pipeline: RenderPipeline,
    vbo: Buffer,
    ebo: Buffer,
    texture_bind_group: BindGroup,
    rt: RenderTexture,
    rt_bind_group: BindGroup,
    texture_initiated: bool,
}

impl State {
    fn new() -> Result<Self, String> {
        let render_pipeline = create_pipeline(Some(RENDER_TEXTURE_FORMAT))?;
        let display_pipeline = create_pipeline(None)?;

        let texture = gfx::create_texture()
            .from_image(include_bytes!("assets/ferris.png"))
            .build()?;

        let sampler = gfx::create_sampler().build()?;

        let texture_bind_group = gfx::create_bind_group()
            .with_layout(render_pipeline.bind_group_layout_ref(0)?)
            .with_texture(0, &texture)
            .with_sampler(1, &sampler)
            .build()?;

        #[rustfmt::skip]
        let vertices: &[f32] = &[
            // Render target source quad.
            0.9,  0.9,     1.0, 1.0,
            0.9, -0.9,     1.0, 0.0,
            -0.9, -0.9,    0.0, 0.0,
            -0.9,  0.9,    0.0, 1.0,

            // Minified render target display quad.
            0.3,  0.3,     1.0, 1.0,
            0.3, -0.3,     1.0, 0.0,
            -0.3, -0.3,    0.0, 0.0,
            -0.3,  0.3,    0.0, 1.0,
        ];
        let vbo = gfx::create_vertex_buffer(vertices).build()?;

        #[rustfmt::skip]
        let indices: &[u16] = &[
            0, 1, 3,
            1, 2, 3,

            4, 5, 7,
            5, 6, 7,
        ];
        let ebo = gfx::create_index_buffer(indices).build()?;

        let rt = gfx::create_render_texture()
            .with_size(texture.width() as _, texture.height() as _)
            .with_format(RENDER_TEXTURE_FORMAT)
            .with_mipmaps()
            .build()?;
        if rt.mip_level_count() != GENERATED_LEVELS {
            return Err("Render texture did not allocate the complete mip chain".to_string());
        }

        let rt_bind_group = gfx::create_bind_group()
            .with_layout(display_pipeline.bind_group_layout_ref(0)?)
            .with_texture(0, rt.texture())
            .with_sampler(1, &sampler)
            .build()?;

        Ok(State {
            render_pipeline,
            display_pipeline,
            vbo,
            ebo,
            texture_bind_group,
            rt,
            rt_bind_group,
            texture_initiated: false,
        })
    }
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
        .unwrap()
}

fn update(state: &mut State) {
    if !state.texture_initiated {
        {
            let renderer = render_texture(
                &state.render_pipeline,
                state,
                &state.texture_bind_group,
                0..6,
                Some(Color::TRANSPARENT),
            );
            gfx::render_to_texture(&state.rt, &renderer).unwrap();
        }
        gfx::generate_mipmaps(state.rt.texture()).unwrap();
        state.texture_initiated = true;
    }

    let renderer = render_texture(
        &state.display_pipeline,
        state,
        &state.rt_bind_group,
        6..12,
        Some(Color::rgb(0.1, 0.2, 0.3)),
    );
    gfx::render_to_frame(&renderer).unwrap();
}

fn create_pipeline(format: Option<TextureFormat>) -> Result<RenderPipeline, String> {
    let pipeline = gfx::create_render_pipeline(SHADER)
        .with_label("Image Pipeline")
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
        .with_blend_mode(BlendMode::NORMAL);

    match format {
        Some(format) => pipeline.with_compatible_texture(format).build(),
        None => pipeline.build(),
    }
}

fn render_texture<'a>(
    pipeline: &'a RenderPipeline,
    state: &'a State,
    bind_group: &'a BindGroup,
    range: Range<u32>,
    clear_color: Option<Color>,
) -> Renderer<'a> {
    let mut renderer = Renderer::new();
    let rpass = renderer.begin_pass();

    if let Some(color) = clear_color {
        rpass.clear_color(color);
    }

    rpass
        .pipeline(pipeline)
        .buffers(&[&state.vbo, &state.ebo])
        .bindings(&[bind_group])
        .draw(range);

    renderer
}
