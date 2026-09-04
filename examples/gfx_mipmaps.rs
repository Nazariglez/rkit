use rkit::app::WindowConfig;
use rkit::gfx::{
    self, BindGroup, BindGroupLayout, BindingType, Buffer, Color, RenderPipeline, Renderer,
    TextureMipLevel, VertexFormat, VertexLayout,
};

const WIDTH: f32 = 1200.0;
const HEIGHT: f32 = 650.0;
const VISIBLE_LEVELS: usize = 5;
const GENERATED_LEVELS: u32 = 9;
const MIP_IMAGES: [&[u8]; VISIBLE_LEVELS] = [
    include_bytes!("assets/ferris-mipmaps.png"),
    include_bytes!("assets/ferris-mipmaps-1.png"),
    include_bytes!("assets/ferris-mipmaps-2.png"),
    include_bytes!("assets/ferris-mipmaps-3.png"),
    include_bytes!("assets/ferris-mipmaps-4.png"),
];

const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
struct MipLevel {
    value: f32,
}
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    return output;
}
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
@group(1) @binding(0) var<uniform> mip_level: MipLevel;
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSampleLevel(image, image_sampler, input.uv, mip_level.value);
    let dark = vec3<f32>(0.03, 0.04, 0.06);
    let light = vec3<f32>(0.82, 0.84, 0.88);
    let background = select(dark, light, input.uv.x > 0.5);
    return vec4<f32>(mix(background, color.rgb, color.a), 1.0);
}
"#;

struct State {
    pipeline: RenderPipeline,
    vertices: Buffer,
    indices: Buffer,
    textures: Vec<BindGroup>,
    levels: Vec<BindGroup>,
}

impl State {
    fn new() -> Result<Self, String> {
        let pipeline = gfx::create_render_pipeline(SHADER)
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
            .with_bind_group_layout(
                BindGroupLayout::new()
                    .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
            )
            .build()?;

        let automatic = gfx::create_texture()
            .from_image(MIP_IMAGES[0])
            .with_mipmaps()
            .build()?;
        if automatic.mip_level_count() != GENERATED_LEVELS {
            return Err("Automatic texture did not allocate the complete mip chain".to_string());
        }
        let mipmaps = MIP_IMAGES
            .iter()
            .map(|bytes| decode_image(bytes))
            .collect::<Result<Vec<_>, _>>()?;
        let levels = mipmaps
            .iter()
            .map(|(pixels, width, height)| TextureMipLevel::new(pixels, *width, *height))
            .collect::<Vec<_>>();
        let manual = gfx::create_texture().from_mipmaps(&levels).build()?;
        if manual.mip_level_count() != VISIBLE_LEVELS as u32 {
            return Err("Manual texture did not preserve the supplied mip prefix".to_string());
        }
        let sampler = gfx::create_sampler().build()?;
        let textures = [&automatic, &manual]
            .into_iter()
            .map(|texture| {
                gfx::create_bind_group()
                    .with_layout(pipeline.bind_group_layout_ref(0)?)
                    .with_texture(0, texture)
                    .with_sampler(1, &sampler)
                    .build()
            })
            .collect::<Result<Vec<_>, String>>()?;
        let levels = (0..VISIBLE_LEVELS)
            .map(|level| {
                let uniform = gfx::create_uniform_buffer(&[level as f32]).build()?;
                gfx::create_bind_group()
                    .with_layout(pipeline.bind_group_layout_ref(1)?)
                    .with_uniform(0, &uniform)
                    .build()
            })
            .collect::<Result<Vec<_>, String>>()?;

        let mut vertex_data = Vec::new();
        let mut index_data = Vec::new();
        for y in [85.0, 380.0] {
            for level in 0..VISIBLE_LEVELS {
                push_quad(
                    &mut vertex_data,
                    &mut index_data,
                    52.0 + level as f32 * 224.0,
                    y,
                    200.0,
                    125.0,
                );
            }
        }

        Ok(Self {
            pipeline,
            vertices: gfx::create_vertex_buffer(&vertex_data).build()?,
            indices: gfx::create_index_buffer(&index_data).build()?,
            textures,
            levels,
        })
    }
}

fn main() -> Result<(), String> {
    rkit::init_with(|| State::new().unwrap())
        .with_window(
            WindowConfig::default()
                .title("Generated mipmaps (top) | Manual mipmaps (bottom) | LOD 0 -> 4")
                .size(WIDTH as u32, HEIGHT as u32)
                .resizable(false),
        )
        .update(update)
        .run()
}

fn update(state: &mut State) {
    let mut renderer = Renderer::new();
    let pass = renderer.begin_pass();
    pass.clear_color(Color::rgb(0.035, 0.04, 0.055));

    for row in 0..state.textures.len() {
        for level in 0..state.levels.len() {
            let index = (row * VISIBLE_LEVELS + level) as u32 * 6;
            pass.begin_command()
                .pipeline(&state.pipeline)
                .buffers(&[&state.vertices, &state.indices])
                .bindings(&[&state.textures[row], &state.levels[level]])
                .draw(index..index + 6);
        }
    }

    gfx::render_to_frame(&renderer).unwrap();
}

fn decode_image(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    let image = image::load_from_memory(bytes)
        .map_err(|err| format!("Could not decode manual mip image: {err}"))?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Ok((image.into_raw(), width, height))
}

fn push_quad(
    vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    let left = x / WIDTH * 2.0 - 1.0;
    let right = (x + width) / WIDTH * 2.0 - 1.0;
    let top = 1.0 - y / HEIGHT * 2.0;
    let bottom = 1.0 - (y + height) / HEIGHT * 2.0;
    let base = (vertices.len() / 4) as u32;
    vertices.extend_from_slice(&[
        left, top, 0.0, 0.0, right, top, 1.0, 0.0, right, bottom, 1.0, 1.0, left, bottom, 0.0, 1.0,
    ]);
    indices.extend_from_slice(&[base, base + 1, base + 3, base + 1, base + 2, base + 3]);
}
