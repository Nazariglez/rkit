use crate::filters::{create_filter_pipeline, Filter};
use crate::gfx;
use crate::gfx::{BindGroup, BindGroupLayout, BindingType, Buffer, RenderPipeline};

// TODO gaussian blur in One pass? Or bettert to use just two passed? how to adapt the filter system?

// // language=wgsl
// const FRAG: &str = r#"
// struct Blur {
//     strength: f32,
//     quality: f32,
// }
//
// @group(1) @binding(0)
// var<uniform> blur: Blur;
//
// @fragment
// fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//     let color = textureSample(t_texture, s_texture, in.uvs);
//     return color;
// }"#;
//
// #[derive(Copy, Clone, Debug, PartialEq)]
// pub struct BlurParams {
//     pub strength: f32,
//     pub quality: f32,
// }
//
// impl Default for BlurParams {
//     fn default() -> Self {
//         Self {
//             strength: 8.0,
//             quality: 4.0,
//         }
//     }
// }
//
// pub struct BlurFilter {
//     pip: RenderPipeline,
//     ubo: Buffer,
//     bind_groups: [BindGroup; 1],
//
//     last_params: BlurParams,
//     pub params: BlurParams,
//
//     pub enabled: bool,
// }
//
// impl BlurFilter {
//     pub fn new(params: BlurParams) -> Result<Self, String> {
//         let pip = create_filter_pipeline(FRAG, |builder| {
//             builder
//                 .with_label("BlurFilter Pipeline")
//                 .with_bind_group_layout(
//                     BindGroupLayout::default()
//                         .with_entry(BindingType::uniform(0).with_fragment_visibility(true)),
//                 )
//                 .build()
//         })?;
//
//         let ubo = gfx::create_uniform_buffer(&ubo_data(&params))
//             .with_label("BlurFilter UBO")
//             .with_write_flag(true)
//             .build()?;
//
//         let bind_group = gfx::create_bind_group()
//             .with_label("BlurFilter BindGroup(1)")
//             .with_layout(pip.bind_group_layout_ref(1)?)
//             .with_uniform(0, &ubo)
//             .build()?;
//
//         Ok(Self {
//             pip,
//             ubo,
//             bind_groups: [bind_group],
//             last_params: params,
//             params,
//
//             enabled: true,
//         })
//     }
// }
//
// impl Filter for BlurFilter {
//     fn is_enabled(&self) -> bool {
//         self.enabled
//     }
//
//     fn update(&mut self) -> Result<(), String> {
//         if self.last_params != self.params {
//             gfx::write_buffer(&self.ubo)
//                 .with_data(&ubo_data(&self.params))
//                 .build()?;
//             self.last_params = self.params;
//         }
//
//         Ok(())
//     }
//
//     fn pipeline(&self) -> &RenderPipeline {
//         &self.pip
//     }
//
//     fn bind_groups(&self) -> &[BindGroup] {
//         &self.bind_groups
//     }
// }
//
// fn ubo_data(params: &BlurParams) -> [f32; 2] {
//     [params.strength, params.quality]
// }
