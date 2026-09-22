fn sprite_sample_to_pm_output(sampled: vec4<f32>, tint: vec4<f32>, source_pm: f32) -> vec4<f32> {
    let source_rgb = sampled.rgb * select(sampled.a, 1.0, source_pm == 1.0);
    return vec4(source_rgb * tint.rgb * tint.a, sampled.a * tint.a);
}
