use rkit::gfx::{self, BindGroup, Color, ComputePipeline, ReadbackTicket, Renderer, Texture};

const SIZE: u32 = 96;
const BAKE: &str = r#"
@group(0) @binding(0) var field: texture_storage_2d<r32float, write>;
@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    let size = textureDimensions(field); if (id.x >= size.x || id.y >= size.y) { return; }
    let uv = vec2f(id.xy) / vec2f(size) - 0.5f;
    textureStore(field, vec2i(id.xy), vec4f(exp(-dot(uv, uv) * 18.0f), 0.0f, 0.0f, 1.0f));
}
"#;

struct State {
    bake: ComputePipeline,
    field: Texture,
    group: BindGroup,
    ticket: Option<ReadbackTicket>,
    color: Color,
    requested: bool,
}
fn main() {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
        .unwrap();
}
impl State {
    fn new() -> Result<Self, String> {
        let field = gfx::create_storage_texture()
            .with_size(SIZE, SIZE)
            .with_format(gfx::TextureFormat::R32Float)
            .build()?;
        let bake = gfx::create_compute_pipeline(BAKE).build()?;
        let group = gfx::create_bind_group()
            .with_layout(bake.bind_group_layout_ref(0)?)
            .with_storage_texture_writeonly(0, &field)
            .build()?;
        Ok(Self {
            bake,
            field,
            group,
            ticket: None,
            color: Color::rgb(0.02, 0.03, 0.07),
            requested: false,
        })
    }
}
fn update(state: &mut State) {
    if !state.requested {
        let mut compute = gfx::Compute::new();
        compute
            .dispatch_for(&state.bake, [SIZE, SIZE, 1])
            .bindings(&[&state.group]);
        gfx::compute(&compute).unwrap();
        state.ticket = Some(gfx::read_texture(&state.field).build().unwrap());
        state.requested = true;
    }
    if let Some(ticket) = &mut state.ticket
        && let Some(gfx::Readback::Texture(snapshot)) = ticket.try_take().unwrap()
    {
        assert_eq!(snapshot.format, gfx::TextureFormat::R32Float);
        assert_eq!((snapshot.width, snapshot.height), (SIZE, SIZE));
        assert_eq!(snapshot.bytes_per_row, snapshot.width * 4);
        let total = snapshot
            .bytes
            .chunks_exact(snapshot.bytes_per_row as usize)
            .flat_map(|row| row.chunks_exact(4))
            .map(|bytes| f32::from_ne_bytes(bytes.try_into().unwrap()))
            .sum::<f32>();
        let mean = total / (snapshot.width * snapshot.height) as f32;
        state.color = Color::rgb(mean, mean * 0.45, 1.0 - mean * 0.3);
        state.ticket = None;
    }
    let mut renderer = Renderer::new();
    renderer.begin_pass().clear_color(state.color);
    gfx::render_to_frame(&renderer).unwrap();
}
