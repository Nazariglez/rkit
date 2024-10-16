const SRGB_THRESHOLD: f32 = 0.04045;
const INV_GAMMA: f32 = 2.4;
const A: f32 = 0.055;

/// Converts a single channel from sRGB to linear space
fn srgb_to_linear_channel(x: f32) -> f32 {
    return select(x / 12.92, pow((x + A) / (1.0 + A), INV_GAMMA), x > SRGB_THRESHOLD);
}

/// Converts an entire color from sRGB to linear space
fn srgb_to_linear(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        srgb_to_linear_channel(color.r),
        srgb_to_linear_channel(color.g),
        srgb_to_linear_channel(color.b),
        color.a
    );
}