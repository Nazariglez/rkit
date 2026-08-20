use super::rich::PixelRect;
use crate::Sprite;
use corelib::{
    gfx::{
        self, BindGroupLayout, BindingType, Buffer, RenderPipeline, RenderTexture, Renderer,
        Sampler, Texture, TextureFilter, TextureFormat, TextureId, TextureWrap, VertexFormat,
        VertexLayout,
    },
    math::{Vec2, vec2},
};
use rustc_hash::FxHashMap;

const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uvs: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uvs: vec2<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4(model.position, 0.0, 1.0);
    out.uvs = model.uvs;
    return out;
}

@group(0) @binding(0)
var source_texture: texture_2d<f32>;
@group(0) @binding(1)
var source_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source_texture, source_sampler, in.uvs);
}
"#;

#[derive(Clone)]
pub(crate) struct IconBake {
    pub(crate) sprite: Sprite,
    pub(crate) frame: PixelRect,
    pub(crate) outer_pos: Vec2,
}

pub(crate) struct IconBaker {
    pipeline: RenderPipeline,
    sampler: Sampler,
    vertices: Buffer,
    temp_vertices: Vec<[f32; 4]>,
}

impl IconBaker {
    pub(crate) fn new() -> Result<Self, String> {
        let pipeline = gfx::create_render_pipeline(SHADER)
            .with_label("Text icon atlas bake pipeline")
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
            .with_compatible_texture(TextureFormat::Rgba8UNormSrgb)
            .build()?;
        let sampler = gfx::create_sampler()
            .with_label("Text icon atlas bake sampler")
            .with_wrap_x(TextureWrap::Clamp)
            .with_wrap_y(TextureWrap::Clamp)
            .with_filter(TextureFilter::Nearest)
            .build()?;
        let vertices = gfx::create_vertex_buffer(&[] as &[[f32; 4]])
            .with_label("Text icon atlas bake vertices")
            .with_write_flag(true)
            .build()?;
        Ok(Self {
            pipeline,
            sampler,
            vertices,
            temp_vertices: Vec::new(),
        })
    }

    pub(crate) fn bake(
        &mut self,
        target: &RenderTexture,
        icons: &[IconBake],
    ) -> Result<(), String> {
        if icons.is_empty() {
            return Ok(());
        }

        let mut grouped: FxHashMap<TextureId, (Texture, Vec<&IconBake>)> = FxHashMap::default();
        for icon in icons {
            let texture = icon.sprite.texture();
            if texture.id() == target.texture().id() {
                return Err("A text icon cannot use its destination atlas as a source".into());
            }
            grouped
                .entry(texture.id())
                .or_insert_with(|| (texture.clone(), Vec::new()))
                .1
                .push(icon);
        }

        self.temp_vertices.clear();
        let mut sources = Vec::with_capacity(grouped.len());
        for (_, (texture, icons)) in grouped {
            let start = self.temp_vertices.len() as u32;
            for icon in icons {
                push_icon_vertices(&mut self.temp_vertices, target.size(), icon);
            }
            sources.push((texture, start..self.temp_vertices.len() as u32));
        }
        gfx::write_buffer(&self.vertices)
            .with_data(&self.temp_vertices)
            .build()?;

        let mut bindings = Vec::with_capacity(sources.len());
        for (texture, _) in &sources {
            bindings.push(
                gfx::create_bind_group()
                    .with_label("Text icon atlas bake source")
                    .with_layout(self.pipeline.bind_group_layout_ref(0)?)
                    .with_texture(0, texture)
                    .with_sampler(1, &self.sampler)
                    .build()?,
            );
        }

        let mut renderer = Renderer::new();
        for ((_, range), binding) in sources.iter().zip(&bindings) {
            renderer
                .begin_pass()
                .pipeline(&self.pipeline)
                .buffers(&[&self.vertices])
                .bindings(&[binding])
                .draw(range.clone());
        }
        gfx::render_to_texture(target, &renderer)
    }
}

fn push_icon_vertices(vertices: &mut Vec<[f32; 4]>, target_size: Vec2, icon: &IconBake) {
    let frame = icon.frame;
    let source_size = icon.sprite.texture().size();
    let origin = frame.origin.as_vec2();
    let size = frame.size.as_vec2();
    let outer = icon.outer_pos;

    let dx = [
        outer.x,
        outer.x + 1.0,
        outer.x + size.x + 1.0,
        outer.x + size.x + 2.0,
    ];
    let dy = [
        outer.y,
        outer.y + 1.0,
        outer.y + size.y + 1.0,
        outer.y + size.y + 2.0,
    ];
    let ux = [
        (origin.x + 0.5) / source_size.x,
        origin.x / source_size.x,
        (origin.x + size.x) / source_size.x,
        (origin.x + size.x - 0.5) / source_size.x,
    ];
    let uy = [
        (origin.y + 0.5) / source_size.y,
        origin.y / source_size.y,
        (origin.y + size.y) / source_size.y,
        (origin.y + size.y - 0.5) / source_size.y,
    ];

    for y in 0..3 {
        for x in 0..3 {
            push_quad(
                vertices,
                target_size,
                vec2(dx[x], dy[y]),
                vec2(dx[x + 1], dy[y + 1]),
                vec2(ux[x], uy[y]),
                vec2(ux[x + 1], uy[y + 1]),
            );
        }
    }
}

fn push_quad(
    vertices: &mut Vec<[f32; 4]>,
    target_size: Vec2,
    min: Vec2,
    max: Vec2,
    uv_min: Vec2,
    uv_max: Vec2,
) {
    let min = to_clip(min, target_size);
    let max = to_clip(max, target_size);
    let top_left = [min.x, min.y, uv_min.x, uv_min.y];
    let top_right = [max.x, min.y, uv_max.x, uv_min.y];
    let bottom_left = [min.x, max.y, uv_min.x, uv_max.y];
    let bottom_right = [max.x, max.y, uv_max.x, uv_max.y];
    vertices.extend_from_slice(&[
        top_left,
        bottom_left,
        bottom_right,
        top_left,
        bottom_right,
        top_right,
    ]);
}

fn to_clip(point: Vec2, target_size: Vec2) -> Vec2 {
    vec2(
        point.x / target_size.x * 2.0 - 1.0,
        1.0 - point.y / target_size.y * 2.0,
    )
}
