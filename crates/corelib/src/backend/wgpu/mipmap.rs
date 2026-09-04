use crate::gfx::TextureFormat;

const SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let x = i32(index) / 2;
    let y = i32(index) & 1;
    let uv = vec2<f32>(f32(x) * 2.0, f32(y) * 2.0);
    var output: VertexOutput;
    output.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    output.uv = uv;
    return output;
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source, source_sampler, input.uv);
}
"#;

pub(crate) struct MipmapGenerator {
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    sampler: wgpu::Sampler,
    pipelines: Vec<(TextureFormat, wgpu::RenderPipeline)>,
}

impl MipmapGenerator {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RKit mipmap bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("RKit mipmap sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RKit mipmap shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RKit mipmap pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        Self {
            layout,
            pipeline_layout,
            shader,
            sampler,
            pipelines: Vec::new(),
        }
    }

    pub(crate) fn validate_format(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        label: Option<&str>,
        format: TextureFormat,
    ) -> Result<(), String> {
        let label = label.map_or(String::new(), |label| format!(" '{label}'"));
        if !matches!(
            format,
            TextureFormat::Rgba8UNorm | TextureFormat::Rgba8UNormSrgb
        ) {
            return Err(format!(
                "Texture{label} format {format:?} does not support generated mipmaps"
            ));
        }

        let raw_format = format.as_wgpu();
        if !device.features().contains(raw_format.required_features()) {
            return Err(format!(
                "Texture{label} format {format:?} is not enabled on the active device"
            ));
        }
        let adapter_features = adapter.get_texture_format_features(raw_format);
        let guaranteed_features = raw_format.guaranteed_format_features(device.features());
        let required = wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST;
        if !adapter_features.allowed_usages.contains(required)
            || !guaranteed_features.allowed_usages.contains(required)
        {
            return Err(format!(
                "Texture{label} format {format:?} lacks operations required for mipmap generation"
            ));
        }

        let filterable = wgpu::TextureFormatFeatureFlags::FILTERABLE;
        if !adapter_features.flags.contains(filterable)
            || !guaranteed_features.flags.contains(filterable)
        {
            return Err(format!(
                "Texture{label} format {format:?} is not filterable for mipmap generation"
            ));
        }
        Ok(())
    }

    fn pipeline(&mut self, device: &wgpu::Device, format: TextureFormat) -> wgpu::RenderPipeline {
        if let Some((_, pipeline)) = self.pipelines.iter().find(|(target, _)| *target == format) {
            return pipeline.clone();
        }

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RKit mipmap pipeline"),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(format.as_wgpu().into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        self.pipelines.push((format, pipeline.clone()));
        pipeline
    }

    pub(crate) fn generate(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        format: TextureFormat,
        mip_level_count: u32,
    ) {
        if mip_level_count <= 1 {
            return;
        }
        let pipeline = self.pipeline(device, format);
        let views = (0..mip_level_count)
            .map(|level| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("RKit mipmap level"),
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RKit mipmap encoder"),
        });
        for target in 1..mip_level_count as usize {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("RKit mipmap bind group"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&views[target - 1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RKit mipmap pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &views[target],
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit(Some(encoder.finish()));
    }
}
