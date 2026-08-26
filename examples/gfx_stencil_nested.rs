use rkit::{
    draw::create_draw_2d,
    gfx::{
        self, Buffer, Color, ColorMask, CompareMode, RenderPipeline, Renderer, Stencil,
        StencilAction, VertexFormat, VertexLayout,
    },
    input::{KeyCode, is_key_down, is_key_pressed},
    math::vec2,
};

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
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4(model.position, 0.0, 1.0);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(in.color, 1.0);
}
"#;

struct State {
    push: RenderPipeline,
    pop: RenderPipeline,
    clipped: RenderPipeline,
    parent_mask: Buffer,
    child_mask: Buffer,
    parent_content: Buffer,
    child_content: Buffer,
    sibling: Buffer,
    child_enabled: bool,
    offset: f32,
}

impl State {
    fn new() -> Result<Self, String> {
        let layout = || {
            VertexLayout::new()
                .with_attr(0, VertexFormat::Float32x2)
                .with_attr(1, VertexFormat::Float32x3)
        };
        let mask_stencil = |pass| Stencil {
            stencil_fail: StencilAction::Keep,
            depth_fail: StencilAction::Keep,
            pass,
            compare: CompareMode::Equal,
            read_mask: 0xff,
            write_mask: 0xff,
            reference: 0,
        };
        let push = gfx::create_render_pipeline(SHADER)
            .with_label("Nested stencil push")
            .with_vertex_layout(layout())
            .with_color_mask(ColorMask::NONE)
            .with_stencil(mask_stencil(StencilAction::Increment))
            .build()?;
        let pop = gfx::create_render_pipeline(SHADER)
            .with_label("Nested stencil pop")
            .with_vertex_layout(layout())
            .with_color_mask(ColorMask::NONE)
            .with_stencil(mask_stencil(StencilAction::Decrement))
            .build()?;
        let content = gfx::create_render_pipeline(SHADER)
            .with_label("Unmasked stencil example content")
            .with_vertex_layout(layout())
            .build()?;
        let clipped = gfx::create_stencil_variant(
            &content,
            Stencil {
                stencil_fail: StencilAction::Keep,
                depth_fail: StencilAction::Keep,
                pass: StencilAction::Keep,
                compare: CompareMode::Equal,
                read_mask: 0xff,
                write_mask: 0x00,
                reference: 0,
            },
        )?;

        Ok(Self {
            push,
            pop,
            clipped,
            parent_mask: buffer(&quad(-0.85, -0.65, 0.75, 0.75, [0.0; 3]))?,
            child_mask: buffer(&quad(-0.35, -0.25, 0.75, 0.65, [0.0; 3]))?,
            parent_content: buffer(&quad(-0.95, -0.55, 1.25, 0.22, [0.15, 0.7, 0.3]))?,
            child_content: buffer(&quad(-0.55, -0.5, 1.25, 0.8, [0.95, 0.45, 0.1]))?,
            sibling: buffer(&quad(0.62, -0.82, 0.25, 0.2, [0.25, 0.55, 1.0]))?,
            child_enabled: true,
            offset: 0.0,
        })
    }
}

fn buffer(vertices: &[f32]) -> Result<Buffer, String> {
    gfx::create_vertex_buffer(vertices).build()
}

fn quad(x: f32, y: f32, width: f32, height: f32, color: [f32; 3]) -> Vec<f32> {
    let [r, g, b] = color;
    vec![
        x,
        y,
        r,
        g,
        b,
        x + width,
        y,
        r,
        g,
        b,
        x + width,
        y + height,
        r,
        g,
        b,
        x,
        y,
        r,
        g,
        b,
        x + width,
        y + height,
        r,
        g,
        b,
        x,
        y + height,
        r,
        g,
        b,
    ]
}

fn main() {
    rkit::init_with(|| State::new().unwrap())
        .update(update)
        .run()
        .unwrap()
}

fn update(state: &mut State) {
    if is_key_pressed(KeyCode::Space) {
        state.child_enabled = !state.child_enabled;
    }
    if is_key_down(KeyCode::ArrowLeft) {
        state.offset = (state.offset - 0.01).max(-0.2);
    }
    if is_key_down(KeyCode::ArrowRight) {
        state.offset = (state.offset + 0.01).min(0.2);
    }

    let mut renderer = Renderer::new();
    let pass = renderer.begin_pass();
    pass.clear_color(Color::rgb(0.04, 0.05, 0.08))
        .clear_stencil(0);
    pass.begin_command()
        .pipeline(&state.push)
        .buffers(&[&state.parent_mask])
        .stencil_reference(0)
        .draw(0..6);
    pass.begin_command()
        .pipeline(&state.clipped)
        .buffers(&[&state.parent_content])
        .stencil_reference(1)
        .draw(0..6);

    if state.child_enabled {
        let min_x = (0.32 + state.offset).clamp(0.0, 1.0);
        let max_x = (0.72 + state.offset).clamp(0.0, 1.0);
        pass.begin_command()
            .pipeline(&state.push)
            .buffers(&[&state.child_mask])
            .stencil_reference(1)
            .normalized_scissors(min_x, 0.25, max_x, 0.75)
            .draw(0..6);
        pass.begin_command()
            .pipeline(&state.clipped)
            .buffers(&[&state.child_content])
            .stencil_reference(2)
            .draw(0..6);
        pass.begin_command()
            .pipeline(&state.pop)
            .buffers(&[&state.child_mask])
            .stencil_reference(2)
            .normalized_scissors(min_x, 0.25, max_x, 0.75)
            .draw(0..6);
    }

    pass.begin_command()
        .pipeline(&state.clipped)
        .buffers(&[&state.parent_content])
        .stencil_reference(1)
        .draw(0..6);
    pass.begin_command()
        .pipeline(&state.pop)
        .buffers(&[&state.parent_mask])
        .stencil_reference(1)
        .draw(0..6);
    pass.begin_command()
        .pipeline(&state.clipped)
        .buffers(&[&state.sibling])
        .stencil_reference(0)
        .draw(0..6);
    gfx::render_to_frame(&renderer).unwrap();

    let mut legend = create_draw_2d();
    legend
        .text("Space: toggle child mask | Left/Right: move child scissor")
        .position(vec2(18.0, 18.0))
        .size(18.0);
    gfx::render_to_frame(&legend).unwrap();
}
