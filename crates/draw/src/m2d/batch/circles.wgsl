const STROKE_SIZE_PX: f32 = 0.0;
const PI: f32             = 3.141592653589793;
const EPSILON: f32        = 1e-6;

struct Locals {
  mvp: mat4x4<f32>,
  fade_at: f32,  // [0..1] when the tail starts turning solid (loadbar)
  aa: f32,       // 0 = hard edges, 1 = normal AA, >1 = wider AA
  _pad: vec2<f32>,
};
@group(0) @binding(0) var<uniform> locals: Locals;

struct VertexInput {
  @location(0) mode: u32,            // 0=fill, 1=stroke, 2=loadbar
  @location(1) center: vec2<f32>,
  @location(2) out_radius: f32,
  @location(3) in_color: vec4<f32>,  // fill inner / load head
  @location(4) out_color: vec4<f32>, // fill outer / stroke / load tail
  @location(5) in_radius: f32,       // stroke = out - in
  @location(6) start_angle: f32,     // radians
  @location(7) end_angle: f32,       // radians
  @location(8) progress: f32,        // [0-1]
};

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) local_pos: vec2<f32>,
  @location(1) @interpolate(flat) mode: u32,
  @location(2) @interpolate(flat) out_radius: f32,
  @location(3) @interpolate(flat) in_color: vec4<f32>,
  @location(4) @interpolate(flat) out_color: vec4<f32>,
  @location(5) @interpolate(flat) in_radius: f32,
  @location(6) @interpolate(flat) start_angle: f32,
  @location(7) @interpolate(flat) end_angle: f32,
  @location(8) @interpolate(flat) progress: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) v_idx: u32, model: VertexInput) -> VertexOutput {
  let quad = array<vec2<f32>, 6>(
    vec2(-1.0, -1.0), vec2( 1.0, -1.0), vec2(-1.0,  1.0),
    vec2(-1.0,  1.0), vec2( 1.0, -1.0), vec2( 1.0,  1.0),
  );
  let local_pos = quad[v_idx];
  let world_pos = model.center + local_pos * model.out_radius;

  var out: VertexOutput;
  out.position    = locals.mvp * vec4(world_pos, 0.0, 1.0);
  out.local_pos   = local_pos;
  out.mode        = model.mode;
  out.out_radius  = model.out_radius;
  out.in_color    = model.in_color;
  out.out_color   = model.out_color;
  out.in_radius   = model.in_radius;
  out.start_angle = model.start_angle;
  out.end_angle   = model.end_angle;
  out.progress    = model.progress;
  return out;
}

fn pos_mod(value: f32, modulus: f32) -> f32 {
  return value - floor(value / modulus) * modulus;
}

fn cover_outer(value: f32, edge: f32, width: f32) -> f32 {
  if (width > 0.0) {
    return 1.0 - smoothstep(edge - width, edge + width, value);
  } else {
    return select(0.0, 1.0, value < edge);
  }
}

fn cover_inner(value: f32, edge: f32, width: f32) -> f32 {
  if (width > 0.0) {
    return smoothstep(edge - width, edge + width, value);
  } else {
    return select(0.0, 1.0, value > edge);
  }
}

fn ring_coverage(radius_norm: f32, inner_norm: f32, width: f32) -> f32 {
  let outer_cover = cover_outer(radius_norm, 1.0, width);
  let inner_cover = cover_inner(radius_norm, inner_norm, width);
  return outer_cover * inner_cover;
}

fn arc_length(start_angle: f32, end_angle: f32) -> f32 {
  let wrapped = pos_mod(end_angle - start_angle, 2.0 * PI);
  return select(wrapped, 2.0 * PI, abs(wrapped) < EPSILON);
}

// srgb_to_linear
{{SRGB_TO_LINEAR}}

fn fs_fill(in: VertexOutput, radius_norm: f32, pixel_width: f32) -> vec4<f32> {
  let aa_width = pixel_width * locals.aa;
  let edge_cover = cover_outer(radius_norm, 1.0, aa_width);
  if (edge_cover <= 0.0) { discard; }
  let radial_mix = clamp(radius_norm, 0.0, 1.0);
  let color      = mix(in.in_color, in.out_color, radial_mix);
  return srgb_to_linear(vec4(color.rgb, color.a * edge_cover));
}

fn fs_stroke(in: VertexOutput, radius_norm: f32, pixel_width: f32) -> vec4<f32> {
  let aa_width = pixel_width * locals.aa;
  let use_const        = (STROKE_SIZE_PX > 0.0) && (in.in_radius <= 0.0);
  let inner_radius_px  = select(in.in_radius, max(0.0, in.out_radius - STROKE_SIZE_PX), use_const);
  let inner_radius_norm= clamp(inner_radius_px / max(in.out_radius, EPSILON), 0.0, 1.0);
  let ring_cover       = ring_coverage(radius_norm, inner_radius_norm, aa_width);

  if (ring_cover <= 0.0) { discard; }
  return srgb_to_linear(vec4(in.out_color.rgb, in.out_color.a * ring_cover));
}

fn fs_loadbar(in: VertexOutput, radius_norm: f32, pixel_width: f32) -> vec4<f32> {
  let radial_aa_width = pixel_width * locals.aa;

  let use_const        = (STROKE_SIZE_PX > 0.0) && (in.in_radius <= 0.0);
  let inner_radius_px  = select(in.in_radius, max(0.0, in.out_radius - STROKE_SIZE_PX), use_const);
  let inner_radius_norm= clamp(inner_radius_px / max(in.out_radius, EPSILON), 0.0, 1.0);
  let ring_cover       = ring_coverage(radius_norm, inner_radius_norm, radial_aa_width);

  var fragment_angle = atan2(in.local_pos.y, in.local_pos.x);
  if (fragment_angle < 0.0) { fragment_angle += 2.0 * PI; }

  let total_arc     = arc_length(in.start_angle, in.end_angle);
  let sweep_angle   = clamp(in.progress, 0.0, 1.0) * total_arc;
  let relative_angle= pos_mod(fragment_angle - in.start_angle, 2.0 * PI);

  let angular_aa_width = fwidth(relative_angle) * locals.aa;
  var angle_cover: f32;
  if (angular_aa_width > 0.0) {
    angle_cover = 1.0 - smoothstep(sweep_angle - angular_aa_width, sweep_angle + angular_aa_width, relative_angle);
  } else {
    angle_cover = select(0.0, 1.0, relative_angle < sweep_angle);
  }

  let cover = ring_cover * angle_cover;
  if (cover <= 0.0) { discard; }

  let has_sweep      = sweep_angle > EPSILON;
  let progress_along = select(0.0, clamp(relative_angle / sweep_angle, 0.0, 1.0), has_sweep);
  let base_rgb       = mix(in.in_color.rgb, in.out_color.rgb, progress_along);

  let fade_at        = clamp(locals.fade_at, 0.0, 1.0);
  let tail_progress  = clamp((clamp(in.progress, 0.0, 1.0) - fade_at) / (1.0 - fade_at), 0.0, 1.0);
  let tail_fade      = tail_progress * tail_progress * tail_progress * tail_progress * tail_progress;
  let final_rgb      = mix(base_rgb, in.out_color.rgb, tail_fade);

  let alpha = mix(in.in_color.a, in.out_color.a, progress_along) * cover;
  return srgb_to_linear(vec4(final_rgb, alpha));
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let radius_norm = length(in.local_pos);
  let pixel_width = fwidth(radius_norm);

  switch (in.mode) {
    case 0u: { return fs_fill(in, radius_norm, pixel_width); }
    case 1u: { return fs_stroke(in, radius_norm, pixel_width); }
    case 2u: { return fs_loadbar(in, radius_norm, pixel_width); }
    default: { return vec4(0.0); }
  }
}
