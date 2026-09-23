use rkit::gfx::{self, BindGroup, Buffer, Color, ComputePipeline, ReadbackTicket, Renderer};

const COMPUTE: &str = r#"
@group(0) @binding(0) var<storage, read_write> events: array<u32>;
@compute @workgroup_size(1) fn cs_main() { events[0] = 1u; }
"#;

struct State {
    compute: ComputePipeline,
    events: Buffer,
    group: BindGroup,
    ticket: Option<ReadbackTicket>,
    flash: f32,
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
        .unwrap();
}

impl State {
    fn new() -> Result<Self, String> {
        let events = gfx::create_storage_buffer(&[0u32]).build()?;
        let compute = gfx::create_compute_pipeline(COMPUTE).build()?;
        let group = gfx::create_bind_group()
            .with_layout(compute.bind_group_layout_ref(0)?)
            .with_storage_readwrite(0, &events)
            .build()?;
        Ok(Self {
            compute,
            events,
            group,
            ticket: None,
            flash: 0.0,
        })
    }
}

fn update(state: &mut State) {
    if let Some(ticket) = &mut state.ticket
        && let Some(gfx::Readback::Buffer(bytes)) = ticket.try_take().unwrap()
    {
        if u32::from_ne_bytes(bytes[..4].try_into().unwrap()) == 1 {
            state.flash = 1.0;
        }
        state.ticket = None;
    }
    if state.ticket.is_none() && state.flash <= 0.0 {
        let mut compute = gfx::Compute::new();
        compute.clear_buffer(&state.events).unwrap();
        compute
            .dispatch_for(&state.compute, [1, 1, 1])
            .bindings(&[&state.group]);
        gfx::compute(&compute).unwrap();
        state.ticket = Some(
            gfx::read_buffer(&state.events)
                .with_byte_range(0..4)
                .build()
                .unwrap(),
        );
    }
    state.flash = (state.flash - 0.025).max(0.0);
    let color = Color::rgb(
        0.04 + state.flash * 0.9,
        0.08 + state.flash * 0.7,
        0.16 + state.flash * 0.2,
    );
    let mut renderer = Renderer::new();
    renderer.begin_pass().clear_color(color);
    gfx::render_to_frame(&renderer).unwrap();
}
