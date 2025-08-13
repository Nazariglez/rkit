struct Locals {
  mvp: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> locals: Locals;

@group(1) @binding(0) var t_texture: texture_2d<f32>;
@group(1) @binding(1) var s_texture: sampler;

struct VertexInput {
  @location(0) center: vec2<f32>,
  @location(1) half_size: vec2<f32>,
  @location(2) color: vec4<f32>,
  @location(3) uv_pos: vec2<f32>,
  @location(4) uv_size: vec2<f32>,
  @location(5) rotation: f32,
  @location(6) _pad: vec3<f32>,
};

struct VertexOutput {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) v_idx: u32, model: VertexInput) -> VertexOutput {
  let quad = array<vec2<f32>, 6>(
    vec2(-1.0, -1.0), vec2( 1.0, -1.0), vec2(-1.0,  1.0),
    vec2(-1.0,  1.0), vec2( 1.0, -1.0), vec2( 1.0,  1.0),
  );

  let local_pos = quad[v_idx];
  let lp = local_pos * model.half_size;

  let c = cos(model.rotation);
  let s = sin(model.rotation);
  let rot = vec2(lp.x * c - lp.y * s, lp.x * s + lp.y * c);

  let world = model.center + rot;

  let pos   = locals.mvp * vec4(world, 0.0, 1.0);
  let norm_uv  = (local_pos * 0.5) + vec2(0.5, 0.5);
  let uv    = model.uv_pos + norm_uv * model.uv_size;

  return VertexOutput(pos, uv, model.color);
}

// srg to linear
{{SRGB_TO_LINEAR}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let color = srgb_to_linear(in.color);
  return textureSample(t_texture, s_texture, in.uv) * color;
}
