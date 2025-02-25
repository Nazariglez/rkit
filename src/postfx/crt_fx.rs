use crate::gfx::{
    self, BindGroup, BindGroupLayout, BindingType, Buffer, RenderPipeline, Renderer, TextureFilter,
};
use crate::math::Vec2;
use crate::postfx::{IOPostFxData, PostFx, create_pfx_pipeline};
use crate::time;
use encase::{ShaderType, UniformBuffer};

// TODO: line number by height?

// language=wgsl
const FRAG: &str = r#"
struct CrtParams {
    scanline_count: f32,
    curvature_amount: f32,
    chromatic_aberration_amount: f32,
    scanline_intensity: f32,
    roll_line_offset: f32,
    roll_speed: f32,
    roll_height: f32,
    scale_factor: f32,
    vignette_width: f32,
    time: f32,
    _pad: vec2<f32>
}

@group(1) @binding(0) var<uniform> params: CrtParams;

// Apply barrel distortion effect to the UV coordinates
fn uv_curve(uv: vec2<f32>) -> vec2<f32> {
    var curved_uv = (uv - vec2<f32>(0.5)) * 2.0;

    // Use curvature_amount to control the strength of the effect
    curved_uv.x *= 1.0 + pow(abs(curved_uv.y) / 4.0, params.curvature_amount);
    curved_uv.y *= 1.0 + pow(abs(curved_uv.x) / 4.0, params.curvature_amount);

    // Slight scaling down to fit the screen better after distortion
    curved_uv /= params.scale_factor;
    return (curved_uv / 2.0) + vec2<f32>(0.5);
}

// Apply a horizontal band displacement effect
fn band_displacement_fixed(uv: vec2<f32>, screen_size: vec2<f32>, line_y: f32, band_height: f32) -> vec2<f32> {
    // Calculate the distance from the center of the band
    let distance_from_center = abs(uv.y * screen_size.y - line_y);

    // Divide the band height into four sections
    let section_height = band_height / 4.0;

    // Determine the offset based on which section the distance falls into
    // Apply `roll_line_offset` only within the band by multiplying with step() check
    let offset =
        (3.0 * step(distance_from_center, section_height) +
         2.0 * (step(section_height, distance_from_center) - step(2.0 * section_height, distance_from_center)) +
         1.0 * (step(2.0 * section_height, distance_from_center) - step(3.0 * section_height, distance_from_center)))
         * step(distance_from_center, band_height) * params.roll_line_offset;

    // Convert offset to normalized screen space and apply it to the UV coordinates
    return uv + vec2<f32>(-offset / screen_size.x, 0.0);
}

// Calculate the Y-position of the rolling line based on time
fn calculate_line_y(screen_size: vec2<f32>, band_height: f32) -> f32 {
    let height = screen_size.y + band_height * 2.0;
    return height - band_height - ((params.time * params.roll_speed * height) % height);
}

// Sample the texture with chromatic aberration effect
fn sample_texture_with_aberration(clamped_uv: vec2<f32>) -> vec4<f32> {
    let aberration_offset = params.chromatic_aberration_amount;
    let r = textureSample(t_texture, s_texture, clamped_uv + vec2<f32>(0.0, aberration_offset)).rgba;
    let g = textureSample(t_texture, s_texture, clamped_uv - vec2<f32>(aberration_offset, 0.0)).rgba;
    let b = textureSample(t_texture, s_texture, clamped_uv - vec2<f32>(aberration_offset, 0.0)).rgba;
    let alpha = max(r.a, max(g.a, b.a));
    let color = vec3<f32>(r.r, g.g, b.b);
    return vec4<f32>(color * alpha, alpha);
}

// Apply the scanline effect to the color based on vertical position
fn apply_scanline_effect(clamped_uv: vec2<f32>, color: vec3<f32>) -> vec3<f32> {
    let PI = 3.14159;
    let s = sin(clamped_uv.y * params.scanline_count * PI * 2.0);
    let scanline_factor = (s * 0.5 + 0.5) * params.scanline_intensity + (1.0 - params.scanline_intensity);
    let scanline = vec3<f32>(pow(scanline_factor, 0.25));
    return color * scanline;
}

// Apply a vignette effect close to the edges of the screen
fn apply_vignette_edge(uv: vec2<f32>, screen_size: vec2<f32>, color: vec3<f32>) -> vec3<f32> {
    // Calculate the distance from each edge (left, right, top, bottom)
    let width = params.vignette_width;
    let left_dist = smoothstep(0.0, width / screen_size.x, uv.x);
    let right_dist = smoothstep(0.0, width / screen_size.x, 1.0 - uv.x);
    let top_dist = smoothstep(0.0, width / screen_size.y, uv.y);
    let bottom_dist = smoothstep(0.0, width / screen_size.y, 1.0 - uv.y);

    // Combine the distances to create a rounded corner vignette effect
    let vignette_factor = left_dist * right_dist * top_dist * bottom_dist;
    // Darken the color based on the vignette factor
    return color * vignette_factor;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen_size = vec2<f32>(textureDimensions(t_texture));

    // Calculate the Y position of the rolling line and apply band displacement
    let line_y = calculate_line_y(screen_size, params.roll_height);
    var displaced_uv = band_displacement_fixed(in.uvs, screen_size, line_y, 300.0);

    // Apply curved UVs for barrel distortion
    let curved_uv = uv_curve(displaced_uv);

    // Clamp UV coordinates and set out-of-bounds mask
    let clamped_uv = clamp(curved_uv, vec2<f32>(0.0), vec2<f32>(1.0));
    let is_in_bounds = step(0.0, curved_uv.x) * step(curved_uv.x, 1.0) *
                       step(0.0, curved_uv.y) * step(curved_uv.y, 1.0);

    // Sample the texture with chromatic aberration
    let color = sample_texture_with_aberration(clamped_uv);

    // Apply scanline effect to the sampled color
    let final_color = apply_scanline_effect(clamped_uv, color.rgb);

    // Apply the vignette effect close to the edges
    let vignetted_color = apply_vignette_edge(clamped_uv, screen_size, final_color);
    return vec4<f32>(vignetted_color * is_in_bounds, color.a);
}
"#;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CrtParams {
    /// Number of scanlines
    pub scanline_count: f32,
    /// Curvature strength
    pub curvature_amount: f32,
    /// Strength of RGB split
    pub chromatic_aberration_amount: f32,
    /// Darkness of scanlines
    pub scanline_intensity: f32,
    /// Horizontal offset the rolling line effect
    pub roll_line_offset: f32,
    /// Speed of the rolling line
    pub roll_speed: f32,
    /// Height of the rolling line
    pub roll_height: f32,
    /// Adjust the scale to avoid or show borders
    pub scale_factor: f32,
    /// Adjust the width of the border's vignette
    pub vignette_width: f32,
}

impl Default for CrtParams {
    fn default() -> Self {
        Self {
            scanline_count: 300.0,
            curvature_amount: 2.9,
            chromatic_aberration_amount: 0.0015,
            scanline_intensity: 0.65,
            roll_line_offset: 2.0,
            roll_speed: 0.04,
            roll_height: 310.0,
            scale_factor: 0.998,
            vignette_width: 8.0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, ShaderType)]
struct InnerParams {
    scanline_count: f32,
    curvature_amount: f32,
    chromatic_aberration_amount: f32,
    scanline_intensity: f32,
    roll_line_offset: f32,
    roll_speed: f32,
    roll_height: f32,
    scale_factor: f32,
    vignette_width: f32,
    time: f32,
    _pad: Vec2,
}

pub struct CrtFx {
    pip: RenderPipeline,
    ubo: Buffer,
    bind_group: BindGroup,
    ubs: UniformBuffer<Vec<u8>>,

    pub params: CrtParams,

    pub enabled: bool,
}

impl CrtFx {
    pub fn new(params: CrtParams) -> Result<Self, String> {
        let pip = create_pfx_pipeline(FRAG, |builder| {
            builder
                .with_label("CRTFx Pipeline")
                .with_bind_group_layout(
                    BindGroupLayout::default()
                        .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
                )
                .build()
        })?;

        // uniform buffer storage
        let mut ubs = UniformBuffer::new(vec![]);
        ubs.write(&ubo_data(&params)).map_err(|e| e.to_string())?;

        let ubo = gfx::create_uniform_buffer(ubs.as_ref())
            .with_label("CRTFx UBO")
            .with_write_flag(true)
            .build()?;

        let bind_group = gfx::create_bind_group()
            .with_label("CRTFx BindGroup(1)")
            .with_layout(pip.bind_group_layout_ref(1)?)
            .with_uniform(0, &ubo)
            .build()?;

        Ok(Self {
            pip,
            ubo,
            bind_group,
            ubs,
            params,
            enabled: true,
        })
    }
}

impl PostFx for CrtFx {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "CRTFx"
    }

    fn apply(&self, data: IOPostFxData) -> Result<bool, String> {
        let mut renderer = Renderer::new();
        renderer
            .begin_pass()
            .pipeline(&self.pip)
            .bindings(&[data.input.bind_group, &self.bind_group])
            .draw(0..6);

        gfx::render_to_texture(data.output.tex, &renderer)?;
        Ok(true)
    }

    fn update(&mut self) -> Result<(), String> {
        self.ubs
            .write(&ubo_data(&self.params))
            .map_err(|e| e.to_string())?;

        gfx::write_buffer(&self.ubo)
            .with_data(self.ubs.as_ref())
            .build()?;

        Ok(())
    }

    fn texture_filter(&self) -> Option<TextureFilter> {
        None
    }
}

fn ubo_data(params: &CrtParams) -> InnerParams {
    InnerParams {
        scanline_count: params.scanline_count,
        curvature_amount: params.curvature_amount,
        chromatic_aberration_amount: params.chromatic_aberration_amount,
        scanline_intensity: params.scanline_intensity,
        roll_line_offset: params.roll_line_offset,
        roll_speed: params.roll_speed,
        roll_height: params.roll_height,
        scale_factor: params.scale_factor,
        vignette_width: params.vignette_width,
        time: time::elapsed_f32(),
        _pad: Vec2::ZERO,
    }
}
