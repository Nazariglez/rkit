#![allow(clippy::arc_with_non_send_sync)]

use super::{
    context::Context,
    frame::DrawFrame,
    offscreen::OffscreenSurfaceData,
    readback::ReadbackManager,
    shader::{create_shader, resolve_compute, resolve_render},
    surface::{Surface, SurfaceCandidate, SurfaceOwner, SurfaceSource},
    utils::{wgpu_depth_stencil, wgpu_shader_visibility},
};
use crate::{
    backend::{
        traits::GfxBackendImpl,
        wgpu::{
            BindGroupInner, BufferBinding, TextureBindingAccess,
            mipmap::MipmapGenerator,
            pipeline::{ComputePipelineInner, PipelineInner, PipelineRecipe, ShaderInterface},
        },
    },
    gfx::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutRef,
        BindType, Buffer, BufferDescriptor, BufferUsage, Color, Compute, ComputeCommand,
        ComputePipeline, ComputePipelineDescriptor, ComputeWorkgroups, GpuStats, InnerBuffer,
        Limits, MAX_BINDING_ENTRIES, ReadbackTicket, RenderCommand, RenderOperation, RenderPass,
        RenderPipeline, RenderPipelineDescriptor, RenderTexture, RenderTextureDescriptor, Renderer,
        SampledTextureType, Sampler, SamplerDescriptor, Scissor, ShaderInput, Stencil,
        StorageTextureAccess, Texture, TextureDescriptor, TextureFormat, TextureId,
        TextureMipLevel, TextureUpload,
        consts::{
            MAX_BIND_GROUPS_PER_PIPELINE, MAX_PIPELINE_COMPATIBLE_TEXTURES,
            SURFACE_DEFAULT_DEPTH_FORMAT,
        },
    },
    math::{UVec2, vec2},
};
use arrayvec::ArrayVec;
use atomic_refcell::AtomicRefCell;
#[cfg(target_arch = "wasm32")]
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, atomic::AtomicU32},
};
use wgpu::{
    BackendOptions, Backends, BufferDescriptor as WBufferDescriptor, Dx12BackendOptions, Extent3d,
    GlBackendOptions, InstanceDescriptor, InstanceFlags, Origin3d, Queue, StoreOp,
    TexelCopyBufferLayout, TextureDimension,
    util::{BufferInitDescriptor, DeviceExt, new_instance_with_webgpu_detection},
};

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
struct WebDisplay;

#[cfg(target_arch = "wasm32")]
impl HasDisplayHandle for WebDisplay {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::web())
    }
}

fn resolve_scissor(scissor: Scissor, width: u32, height: u32) -> Result<[u32; 4], String> {
    match scissor {
        Scissor::Physical([x, y, scissor_width, scissor_height]) => {
            let max_x = x
                .checked_add(scissor_width)
                .ok_or_else(|| "Physical scissor edges overflow".to_string())?;
            let max_y = y
                .checked_add(scissor_height)
                .ok_or_else(|| "Physical scissor edges overflow".to_string())?;
            if scissor_width == 0 || scissor_height == 0 || max_x > width || max_y > height {
                return Err("Physical scissor must have positive in-target area".to_string());
            }
            Ok([x, y, scissor_width, scissor_height])
        }
        Scissor::Normalized(edges) => {
            if !edges.iter().all(|edge| edge.is_finite()) {
                return Err("Normalized scissor edges must be finite".to_string());
            }

            let [min_x, min_y, max_x, max_y] = edges;
            if max_x <= min_x || max_y <= min_y {
                return Err("Normalized scissor must have positive area".to_string());
            }

            let min_x = (min_x * width as f32).floor().clamp(0.0, width as f32) as u32;
            let min_y = (min_y * height as f32).floor().clamp(0.0, height as f32) as u32;
            let max_x = (max_x * width as f32).ceil().clamp(0.0, width as f32) as u32;
            let max_y = (max_y * height as f32).ceil().clamp(0.0, height as f32) as u32;
            if max_x <= min_x || max_y <= min_y {
                return Err("Normalized scissor does not intersect the render target".to_string());
            }

            Ok([
                min_x,
                min_y,
                max_x.saturating_sub(min_x),
                max_y.saturating_sub(min_y),
            ])
        }
    }
}

fn wgpu_sample_type(sample_type: SampledTextureType) -> wgpu::TextureSampleType {
    match sample_type {
        SampledTextureType::Float { filterable } => wgpu::TextureSampleType::Float { filterable },
        SampledTextureType::Sint => wgpu::TextureSampleType::Sint,
        SampledTextureType::Uint => wgpu::TextureSampleType::Uint,
    }
}

fn wgpu_storage_texture_access(access: StorageTextureAccess) -> wgpu::StorageTextureAccess {
    match access {
        StorageTextureAccess::Readonly => wgpu::StorageTextureAccess::ReadOnly,
        StorageTextureAccess::Writeonly => wgpu::StorageTextureAccess::WriteOnly,
        StorageTextureAccess::Readwrite => wgpu::StorageTextureAccess::ReadWrite,
    }
}

fn pass_uses_depth_stencil(pass: &RenderPass<'_>) -> Result<(bool, bool), String> {
    let has_clear_depth = pass.clear_options.depth.is_some();
    let uses_depth = has_clear_depth
        || pass.commands.iter().any(|command| {
            command
                .pipeline
                .is_some_and(|pipeline| pipeline.inner.uses_depth)
        });
    let has_clear_stencil = pass.clear_options.stencil.is_some();
    let uses_stencil = has_clear_stencil
        || pass.commands.iter().any(|command| {
            command
                .pipeline
                .is_some_and(|pipeline| pipeline.inner.uses_stencil)
        });
    let has_attachment = uses_depth || uses_stencil;
    let has_incompatible_pipeline = has_attachment
        && pass.commands.iter().any(|command| {
            command
                .pipeline
                .is_some_and(|pipeline| !pipeline.inner.uses_depth && !pipeline.inner.uses_stencil)
        });
    if has_incompatible_pipeline {
        return Err(
            "A render pass cannot mix pipelines with and without depth-stencil attachments"
                .to_string(),
        );
    }
    Ok((uses_depth, uses_stencil))
}

fn binding_type_covers(declared: BindType, required: BindType) -> bool {
    matches!(
        (declared, required),
        (
            BindType::Texture(SampledTextureType::Float { filterable: true }),
            BindType::Texture(SampledTextureType::Float { filterable: false }),
        )
    ) || declared == required
}

fn validate_pipeline_bindings(
    pipeline: &str,
    layouts: &[BindGroupLayoutRef],
    interface: &ShaderInterface,
    bind_groups: &[&BindGroup],
) -> Result<(), String> {
    if bind_groups.len() != layouts.len() {
        return Err(format!(
            "{pipeline} requires {} bind groups, but the command binds {}",
            layouts.len(),
            bind_groups.len()
        ));
    }
    for (index, (bind_group, layout)) in bind_groups.iter().zip(layouts).enumerate() {
        if bind_group.inner.layout != layout.id {
            return Err(format!(
                "{pipeline} bind group {index} does not use the pipeline layout"
            ));
        }
    }
    for group in &interface.groups {
        let bind_group = bind_groups
            .get(group.group as usize)
            .ok_or_else(|| format!("{pipeline} requires bind group {}", group.group))?;
        for requirement in &group.bindings {
            let Some(minimum) = requirement.min_buffer_size else {
                continue;
            };
            let available = bind_group
                .inner
                .buffers
                .iter()
                .find(|buffer| buffer.location == requirement.binding.location)
                .map(|buffer| buffer.size)
                .ok_or_else(|| {
                    format!(
                        "{pipeline} @group({}) @binding({}) requires a buffer binding",
                        group.group, requirement.binding.location
                    )
                })?;
            if available < minimum {
                return Err(format!(
                    "{pipeline} @group({}) @binding({}) requires at least {minimum} bytes, but the bound buffer has {available} bytes",
                    group.group, requirement.binding.location
                ));
            }
        }
    }
    Ok(())
}

fn command_has_draws(command: &RenderCommand<'_>) -> bool {
    command.operations.iter().any(|operation| match operation {
        RenderOperation::Direct(vertices) => !vertices.range.is_empty(),
        RenderOperation::Indirect(_) => true,
    })
}

fn track_render_buffer(
    buffers: &mut HashMap<crate::gfx::BufferId, bool>,
    id: crate::gfx::BufferId,
    writable: bool,
) -> Result<(), String> {
    if buffers
        .insert(id, writable)
        .is_some_and(|previous| previous != writable)
    {
        return Err(
            "Render pass aliases a buffer across read-only and writable usages".to_string(),
        );
    }
    Ok(())
}

fn track_render_texture(
    textures: &mut HashMap<TextureId, TextureBindingAccess>,
    id: TextureId,
    access: TextureBindingAccess,
) -> Result<(), String> {
    if textures
        .insert(id, access)
        .is_some_and(|previous| access.conflicts_with(previous))
    {
        return Err("Render pass aliases a texture across incompatible usages".to_string());
    }
    Ok(())
}

fn validate_render_resource_aliases(pass: &RenderPass<'_>) -> Result<(), String> {
    let mut buffers = HashMap::new();
    let mut textures = HashMap::new();

    for command in &pass.commands {
        if command.pipeline.is_none() || !command_has_draws(command) {
            continue;
        }
        for operation in &command.operations {
            if let RenderOperation::Indirect(arguments) = operation {
                track_render_buffer(&mut buffers, arguments.id(), false)?;
            }
        }
        for group in &command.bind_groups {
            for binding in &group.inner.buffers {
                track_render_buffer(&mut buffers, binding.buffer.id(), binding.writable)?;
            }
            for binding in &group.inner.textures {
                track_render_texture(&mut textures, binding.texture.id(), binding.access)?;
            }
        }
    }
    Ok(())
}

fn mark_rendered_storage_textures(renderer: &Renderer) {
    let mut written = HashSet::new();
    for pass in &renderer.passes {
        for command in &pass.commands {
            if command.pipeline.is_none() || !command_has_draws(command) {
                continue;
            }
            for group in &command.bind_groups {
                for binding in &group.inner.textures {
                    if binding.access.writable() && written.insert(binding.texture.id()) {
                        binding.texture.mark_written();
                    }
                }
            }
        }
    }
}

fn encode_render_commands(
    pass: &mut wgpu::RenderPass<'_>,
    commands: &[RenderCommand<'_>],
    width: u32,
    height: u32,
) -> Result<(), String> {
    for command in commands {
        let Some(pipeline) = command.pipeline else {
            continue;
        };
        if !command_has_draws(command) {
            continue;
        }

        let expected_vertex_buffers = pipeline.inner.recipe.vertex_layout.len();
        let vertex_buffers = command
            .buffers
            .iter()
            .filter(|(buffer, _)| matches!(buffer.usage, BufferUsage::Vertex))
            .count();
        if vertex_buffers != expected_vertex_buffers {
            return Err(format!(
                "Render pipeline requires {expected_vertex_buffers} vertex buffers, but the command binds {vertex_buffers}"
            ));
        }
        validate_pipeline_bindings(
            "Render pipeline",
            &pipeline.inner.bind_group_layout,
            &pipeline.inner.interface,
            &command.bind_groups,
        )?;

        pass.set_pipeline(&pipeline.inner.raw);
        let viewport = command.viewport.unwrap_or(crate::gfx::Viewport {
            origin: [0, 0],
            size: vec2(width as f32, height as f32),
        });
        if !viewport.size.is_finite() || viewport.size.x <= 0.0 || viewport.size.y <= 0.0 {
            return Err("Render command viewport size must be finite and positive".to_string());
        }
        let max_x = viewport.origin[0] as f32 + viewport.size.x;
        let max_y = viewport.origin[1] as f32 + viewport.size.y;
        if max_x > width as f32 || max_y > height as f32 {
            return Err("Render command viewport exceeds the render target".to_string());
        }
        pass.set_viewport(
            viewport.origin[0] as f32,
            viewport.origin[1] as f32,
            viewport.size.x,
            viewport.size.y,
            0.0,
            1.0,
        );

        let [x, y, scissor_width, scissor_height] = match command.scissors {
            Some(scissor) => resolve_scissor(scissor, width, height)?,
            None => [0, 0, width, height],
        };
        pass.set_scissor_rect(x, y, scissor_width, scissor_height);
        pass.set_stencil_reference(u32::from(command.stencil_ref.unwrap_or(0)));

        let mut vertex_slot = 0;
        let mut indexed = false;
        for (buffer, range) in &command.buffers {
            if range.is_empty() {
                return Err(format!(
                    "Buffer '{} - ({:?})' range cannot be empty",
                    buffer.inner_label, buffer.id
                ));
            }

            match buffer.usage {
                BufferUsage::Vertex => {
                    pass.set_vertex_buffer(
                        vertex_slot,
                        buffer.inner.borrow().raw.slice(range.clone()),
                    );
                    vertex_slot += 1;
                }
                BufferUsage::Index => {
                    debug_assert!(!indexed, "Cannot bind more than one Index buffer");
                    indexed = true;
                    pass.set_index_buffer(
                        buffer.inner.borrow().raw.slice(range.clone()),
                        pipeline.inner.index_format,
                    );
                }
                BufferUsage::Uniform | BufferUsage::Storage => {}
            }
        }

        for (index, bind_group) in command.bind_groups.iter().enumerate() {
            let index = u32::try_from(index)
                .map_err(|_| "Bind group index exceeds the supported range".to_string())?;
            pass.set_bind_group(index, &bind_group.inner.raw, &[]);
        }

        for operation in &command.operations {
            match operation {
                RenderOperation::Direct(vertices) => {
                    if vertices.range.is_empty() {
                        continue;
                    }
                    let instances = 0..vertices.instances.unwrap_or(1);
                    if indexed {
                        pass.draw_indexed(vertices.range.clone(), 0, instances);
                    } else {
                        pass.draw(vertices.range.clone(), instances);
                    }
                }
                RenderOperation::Indirect(arguments) => {
                    pass.draw_indirect(&arguments.as_ref().inner.borrow().raw, 0);
                }
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_wine() -> bool {
    use windows::{
        Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress},
        core::s,
    };

    // SAFETY: This only queries the handle for a system module already loaded by the process.
    let Ok(ntdll) = (unsafe { GetModuleHandleA(s!("ntdll.dll")) }) else {
        return false;
    };

    // SAFETY: This only checks whether the export exists; the returned pointer is not retained or called.
    unsafe { GetProcAddress(ntdll, s!("wine_get_version")) }.is_some()
}

#[cfg(target_arch = "wasm32")]
async fn init_wasm_gfx(
    source: SurfaceSource,
    vsync: bool,
    win_size: UVec2,
    pixelated: bool,
    backend_override: Option<Backends>,
) -> Result<GfxBackend, String> {
    let backends = backend_override.unwrap_or_default();
    match GfxBackend::new(source.clone(), vsync, win_size, pixelated, backends).await {
        Ok(gfx) => Ok(gfx),
        #[cfg(feature = "webgl")]
        Err(error) => {
            log::error!("Error initializing Gfx backend: {error}");
            log::info!("Fallback to WebGL");
            GfxBackend::new(source, vsync, win_size, pixelated, Backends::GL).await
        }
        #[cfg(not(feature = "webgl"))]
        Err(error) => {
            log::error!("Error initializing Gfx backend: {error}");
            Err(error)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn init_native_gfx(
    source: SurfaceSource,
    vsync: bool,
    win_size: UVec2,
    pixelated: bool,
    backend_override: Option<Backends>,
) -> Result<GfxBackend, String> {
    let backends = match backend_override {
        Some(backends) => {
            log::info!("Using explicit graphics backend mask: {backends:?}");
            backends
        }
        None => {
            #[cfg(target_os = "windows")]
            {
                if is_wine() {
                    log::info!(
                        "Wine-compatible runtime detected; using default graphics backend selection"
                    );
                    Backends::default()
                } else {
                    log::info!("Using ordered Windows graphics backend policy");
                    let attempts = [Backends::DX12, Backends::VULKAN, Backends::GL];
                    let mut errors = Vec::with_capacity(attempts.len());

                    for backends in attempts {
                        log::info!("Attempting graphics backend: {backends:?}");
                        match GfxBackend::new(source.clone(), vsync, win_size, pixelated, backends)
                            .await
                        {
                            Ok(gfx) => {
                                log::info!(
                                    "Successfully initialized graphics backend: {backends:?}"
                                );
                                return Ok(gfx);
                            }
                            Err(error) => {
                                log::warn!(
                                    "Graphics backend initialization failed: {backends:?}: {error}"
                                );
                                errors.push(format!("{backends:?}: {error}"));
                            }
                        }
                    }

                    let error = format!(
                        "All ordered Windows graphics backend attempts failed:\n{}",
                        errors.join("\n")
                    );
                    log::error!("Error initializing Gfx backend: {error}");
                    return Err(error);
                }
            }

            #[cfg(not(target_os = "windows"))]
            {
                Backends::default()
            }
        }
    };

    GfxBackend::new(source, vsync, win_size, pixelated, backends)
        .await
        .map_err(|error| {
            log::error!("Error initializing Gfx backend: {error}");
            error
        })
}

#[derive(Hash, Eq, PartialEq)]
struct LayoutShape {
    entries: ArrayVec<crate::gfx::BindingType, MAX_BINDING_ENTRIES>,
}

pub(crate) struct GfxBackend {
    pub(crate) surface: Surface, // Eventually we could have a HashMap<WindowId, Surface> if we want multiple window

    next_resource_id: u64,
    layout_cache: HashMap<LayoutShape, BindGroupLayoutRef>,
    ctx: Context,

    #[cfg(native_windowed)]
    depth_format: TextureFormat,
    frame: Option<DrawFrame>,

    // used as intermediate for surface and pipeline texture formats
    offscreen: Option<OffscreenSurfaceData>,
    mipmap_generator: MipmapGenerator,
    readbacks: ReadbackManager,

    last_frame_stats: GpuStats,
    current_stats: GpuStats,
}

// This is a hack for wasm32 browsers where there is no threads
#[cfg(target_arch = "wasm32")]
unsafe impl Send for GfxBackend {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for GfxBackend {}

impl GfxBackendImpl for GfxBackend {
    fn frame_size(&self) -> UVec2 {
        self.offscreen
            .as_ref()
            .map(|surface| surface.texture.size().as_uvec2())
            .unwrap_or(UVec2::ZERO)
    }

    fn prepare_frame(&mut self) -> Result<(), String> {
        self.progress_readbacks();
        let can_render = self.surface.config.width > 0 && self.surface.config.height > 0;
        if !can_render {
            // on win_os minized windows can report 0 size, skip rendering
            return Ok(());
        }

        self.push_frame()
    }

    fn progress_readbacks(&mut self) -> bool {
        self.readbacks
            .progress(&self.ctx.device, self.ctx.device_loss())
    }

    fn cancel_readbacks(&mut self) {
        self.readbacks.shutdown();
    }

    fn present_frame(&mut self) {
        self.present_to_screen();

        // new stats
        self.last_frame_stats = std::mem::take(&mut self.current_stats);
    }

    fn render(&mut self, renderer: &Renderer) -> Result<(), String> {
        let offscreen = self
            .offscreen
            .take()
            .ok_or_else(|| "Invalid Offscreen surface".to_string())?;
        let can_render = offscreen.texture.size.x > 0.0 && offscreen.texture.size.y > 0.0;
        let rendered = if can_render {
            self.render_to(&offscreen.texture, renderer)
        } else {
            Ok(())
        };
        self.offscreen = Some(offscreen);
        rendered?;

        if !renderer.passes.is_empty()
            && let Some(frame) = &mut self.frame
        {
            frame.dirty = true;
        }

        Ok(())
    }

    fn render_to(&mut self, texture: &RenderTexture, renderer: &Renderer) -> Result<(), String> {
        debug_assert!(
            texture.texture.write,
            "Cannot write data to a static render texture"
        );

        if texture.size.x <= 0.0 || texture.size.y <= 0.0 {
            return Ok(());
        }

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RenderTexture Encoder"),
            });

        for pass in &renderer.passes {
            validate_render_resource_aliases(pass)?;
            let (uses_depth, uses_stencil) = pass_uses_depth_stencil(pass)?;
            if (uses_depth || uses_stencil) && texture.depth_texture.is_none() {
                return Err("Depth texture is required for depth or stencil testing".to_string());
            }

            let color = Some(wgpu::RenderPassColorAttachment {
                view: &texture.color_attachment,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: pass
                        .clear_options
                        .color
                        .map_or(wgpu::LoadOp::Load, |color| {
                            wgpu::LoadOp::Clear(color.as_wgpu())
                        }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            });
            let depth = uses_depth.then(|| wgpu::Operations {
                load: pass
                    .clear_options
                    .depth
                    .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                store: StoreOp::Store,
            });
            let stencil = uses_stencil.then(|| wgpu::Operations {
                load: pass
                    .clear_options
                    .stencil
                    .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                store: StoreOp::Store,
            });
            let depth_stencil_attachment = texture.depth_texture.as_ref().and_then(|texture| {
                (depth.is_some() || stencil.is_some()).then_some(
                    wgpu::RenderPassDepthStencilAttachment {
                        view: &texture.view,
                        depth_ops: depth,
                        stencil_ops: stencil,
                    },
                )
            });

            let mut raw_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[color],
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let size = texture.texture.raw.size();
            encode_render_commands(&mut raw_pass, &pass.commands, size.width, size.height)?;
        }

        if !renderer.passes.is_empty() {
            self.ctx.queue.submit(Some(encoder.finish()));
            mark_rendered_storage_textures(renderer);
            texture.texture.mark_written();
            self.current_stats.draw_calls += 1;
        }
        Ok(())
    }

    fn create_shader(&mut self, source: &str) -> Result<crate::gfx::Shader, String> {
        create_shader(&self.ctx.device, source)
    }

    fn create_render_pipeline(
        &mut self,
        desc: RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, String> {
        log::debug!("Creating RenderPipeline (label={:?})", desc.label);
        let shader = self.resolve_shader(desc.shader)?;
        let vertex_entry = desc.vs_entry.unwrap_or("vs_main");
        let fragment_entry = desc.fs_entry.unwrap_or("fs_main");
        let interface = resolve_render(&shader, desc.label, vertex_entry, fragment_entry)?;
        let bind_group_layout =
            self.resolve_layouts(desc.label, &interface, &desc.bind_group_layout)?;

        let mut compatible_formats = desc
            .compatible_textures
            .iter()
            .map(|format| format.as_wgpu())
            .collect::<ArrayVec<_, MAX_PIPELINE_COMPATIBLE_TEXTURES>>();
        if compatible_formats.is_empty() {
            compatible_formats.push(self.surface.config.format);
        }

        let blend = desc.blend_mode.map(|mode| mode.as_wgpu());
        let write_mask = desc.color_mask.as_wgpu();
        let targets = compatible_formats
            .iter()
            .map(|format| {
                Some(wgpu::ColorTargetState {
                    format: *format,
                    blend,
                    write_mask,
                })
            })
            .collect();

        let recipe = PipelineRecipe {
            label: desc.label.map(str::to_owned),
            shader: (*shader.raw).clone(),
            vertex_layout: desc.vertex_layout,
            primitive: desc.primitive,
            cull_mode: desc.cull_mode,
            depth: desc.depth_stencil,
            stencil: desc.stencil,
            targets,
            vs_entry: vertex_entry.to_owned(),
            fs_entry: fragment_entry.to_owned(),
        };
        let raw = self.build_render_pipeline(&recipe, &bind_group_layout);

        Ok(RenderPipeline {
            inner: Arc::new(PipelineInner {
                id: resource_id(&mut self.next_resource_id),
                raw,
                index_format: desc.index_format.as_wgpu(),
                uses_depth: recipe.depth.is_some(),
                uses_stencil: recipe.stencil.is_some(),
                bind_group_layout,
                interface,
                recipe,
            }),
        })
    }

    fn create_compute_pipeline(
        &mut self,
        desc: ComputePipelineDescriptor,
    ) -> Result<ComputePipeline, String> {
        if !self.supports_compute() {
            return Err("Compute pipelines are unsupported by this graphics backend".to_string());
        }
        let shader = self.resolve_shader(desc.shader)?;
        let entry = desc.entry.unwrap_or("cs_main");
        let (interface, workgroup_size, workgroup_storage_size) =
            resolve_compute(&shader, desc.label, entry)?;
        self.validate_workgroup_requirements(
            desc.label,
            entry,
            workgroup_size,
            workgroup_storage_size,
        )?;
        let bind_group_layout =
            self.resolve_layouts(desc.label, &interface, &Default::default())?;
        let layout_refs = bind_group_layout
            .iter()
            .map(|layout| Some(layout.raw.as_ref()))
            .collect::<Vec<_>>();
        let layout = self
            .ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: desc.label,
                bind_group_layouts: &layout_refs,
                immediate_size: 0,
            });
        let raw = self
            .ctx
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: desc.label,
                layout: Some(&layout),
                module: &shader.raw,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            });
        Ok(ComputePipeline {
            inner: Arc::new(ComputePipelineInner {
                raw,
                bind_group_layout,
                interface,
                workgroup_size,
            }),
        })
    }

    fn create_stencil_variant(
        &mut self,
        base: &RenderPipeline,
        stencil: Stencil,
    ) -> Result<RenderPipeline, String> {
        let mut recipe = base.inner.recipe.clone();
        recipe.stencil = Some(stencil);
        let raw = self.build_render_pipeline(&recipe, &base.inner.bind_group_layout);

        Ok(RenderPipeline {
            inner: Arc::new(PipelineInner {
                id: resource_id(&mut self.next_resource_id),
                raw,
                index_format: base.inner.index_format,
                uses_depth: recipe.depth.is_some(),
                uses_stencil: true,
                bind_group_layout: base.inner.bind_group_layout.clone(),
                interface: base.inner.interface.clone(),
                recipe,
            }),
        })
    }

    fn create_buffer(&mut self, desc: BufferDescriptor) -> Result<Buffer, String> {
        log::trace!(
            "Creating Buffer (label={:?}, usage={:?})",
            desc.label,
            desc.usage
        );
        let size = desc.allocation_size.unwrap_or_else(|| {
            if desc.content.is_empty() {
                1024
            } else {
                desc.content.len()
            }
        });
        if size == 0 {
            return Err("Buffer allocations must have a nonzero size".to_string());
        }
        if desc.content.len() > size {
            return Err("Buffer content exceeds its requested allocation size".to_string());
        }
        self.validate_buffer_allocation(desc.usage, size)?;
        if desc.indirect && !self.supports_indirect_execution() {
            return Err("Indirect execution is unsupported by this graphics backend".to_string());
        }

        let mut usage = desc.usage.as_wgpu();
        if desc.write {
            usage |= wgpu::BufferUsages::COPY_DST;
        }
        if desc.indirect {
            usage |= wgpu::BufferUsages::INDIRECT;
        }

        let raw = if desc.content.len() == size {
            self.ctx.device.create_buffer_init(&BufferInitDescriptor {
                label: desc.label,
                contents: desc.content,
                usage,
            })
        } else {
            self.ctx.device.create_buffer(&WBufferDescriptor {
                label: desc.label,
                size: size as _,
                usage,
                mapped_at_creation: false,
            })
        };
        if !desc.content.is_empty() && desc.content.len() != size {
            self.ctx.queue.write_buffer(&raw, 0, desc.content);
        }

        Ok(Buffer {
            id: resource_id(&mut self.next_resource_id),
            inner: Arc::new(AtomicRefCell::new(InnerBuffer {
                size,
                raw: Arc::new(raw),
            })),
            usage: desc.usage,
            write: desc.write,
            inner_label: Arc::new(desc.label.map_or_else(|| "".to_string(), |l| l.to_string())),
        })
    }

    fn create_bind_group(&mut self, desc: BindGroupDescriptor) -> Result<BindGroup, String> {
        let mut next_resource_id = self.next_resource_id;
        let bind_group = self.prepare_bind_group(&mut next_resource_id, desc)?;
        self.next_resource_id = next_resource_id;
        Ok(bind_group)
    }

    fn write_buffer(&mut self, buffer: &Buffer, offset: u64, data: &[u8]) -> Result<(), String> {
        let required = crate::gfx::validate_buffer_write(
            buffer,
            offset,
            data.len(),
            crate::gfx::BufferWriteMode::Immediate,
        )?;
        let offset = usize::try_from(offset)
            .map_err(|_| "Buffer write offset does not fit this platform".to_string())?;
        self.validate_buffer_allocation(buffer.usage, required)?;

        if buffer.size() < required {
            let next_size = next_buffer_size(buffer.size(), required);
            self.validate_buffer_allocation(buffer.usage, next_size)?;
            log::debug!(
                "Updating Buffer '{}' size from {} to {}",
                buffer.inner_label,
                buffer.size(),
                next_size
            );
            let mut usage = buffer.usage.as_wgpu();
            usage |= wgpu::BufferUsages::COPY_DST;

            let raw = self.ctx.device.create_buffer(&WBufferDescriptor {
                label: buffer.inner_label.as_str().into(),
                size: next_size as _,
                usage,
                mapped_at_creation: false,
            });

            let copied = buffer.size().min(offset);
            if copied > 0 {
                log::debug!(
                    "Copying Buffer '{}' memory until offset {}",
                    buffer.inner_label,
                    copied
                );
                let mut encoder =
                    self.ctx
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Buffer Copy Encoder"),
                        });

                encoder.copy_buffer_to_buffer(
                    &buffer.inner.borrow().raw,
                    0,
                    &raw,
                    0,
                    copied as u64,
                );

                self.ctx.queue.submit(Some(encoder.finish()));
                self.current_stats.draw_calls += 1;
            }

            *buffer.inner.borrow_mut() = InnerBuffer {
                size: next_size,
                raw: Arc::new(raw),
            };
        }

        self.ctx
            .queue
            .write_buffer(&buffer.inner.borrow().raw, offset as _, data);
        Ok(())
    }

    fn read_buffer(
        &mut self,
        buffer: &Buffer,
        bytes: std::ops::Range<u64>,
    ) -> Result<ReadbackTicket, String> {
        if let Some(error) = self.ctx.device_loss() {
            self.readbacks.device_lost(error.clone());
            return Err(error);
        }
        if !matches!(buffer.usage(), BufferUsage::Uniform | BufferUsage::Storage) {
            return Err("Buffer was not created with readback support".to_string());
        }
        crate::gfx::validate_buffer_range(buffer, bytes.clone(), "Buffer readback")?;
        let source = buffer.inner.borrow();
        self.readbacks.read_buffer(
            &self.ctx.device,
            &self.ctx.queue,
            &source.raw,
            bytes.start,
            bytes.end - bytes.start,
        )
    }

    fn read_texture(&mut self, texture: &Texture) -> Result<ReadbackTicket, String> {
        if let Some(error) = self.ctx.device_loss() {
            self.readbacks.device_lost(error.clone());
            return Err(error);
        }
        if texture.format.is_depth() || !texture.copyable {
            return Err("Texture was not created with readback support".to_string());
        }
        let width = texture.width() as u32;
        let height = texture.height() as u32;
        let bytes_per_texel = texture
            .format
            .bytes_per_texel()
            .ok_or_else(|| "Texture format does not support readback".to_string())?;
        let tight_row = width
            .checked_mul(bytes_per_texel)
            .ok_or_else(|| "Texture readback row size overflows".to_string())?;
        self.readbacks.read_texture(
            &self.ctx.device,
            &self.ctx.queue,
            &texture.raw,
            width,
            height,
            texture.format,
            tight_row,
        )
    }

    fn compute(&mut self, compute: &Compute<'_>) -> Result<(), String> {
        if let Some(error) = &compute.error {
            return Err(error.clone());
        }
        for command in &compute.commands {
            match command {
                ComputeCommand::Write {
                    buffer,
                    offset,
                    bytes,
                } => {
                    crate::gfx::validate_buffer_write(
                        buffer,
                        *offset,
                        bytes.len(),
                        crate::gfx::BufferWriteMode::Ordered,
                    )?;
                }
                ComputeCommand::Clear { buffer, bytes } => {
                    crate::gfx::validate_buffer_clear(buffer, bytes.clone())?;
                }
                ComputeCommand::Dispatch(dispatch) => self.validate_compute_dispatch(dispatch)?,
            }
        }
        if compute.commands.is_empty() {
            return Ok(());
        }

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Commands Encoder"),
            });
        let mut staging = Vec::new();
        let mut written_textures = HashMap::new();
        for command in &compute.commands {
            match command {
                ComputeCommand::Write {
                    buffer,
                    offset,
                    bytes,
                } => {
                    let source = self.ctx.device.create_buffer_init(&BufferInitDescriptor {
                        label: Some("Compute Buffer Upload"),
                        contents: bytes,
                        usage: wgpu::BufferUsages::COPY_SRC,
                    });
                    encoder.copy_buffer_to_buffer(
                        &source,
                        0,
                        &buffer.inner.borrow().raw,
                        *offset,
                        bytes.len() as u64,
                    );
                    staging.push(source);
                }
                ComputeCommand::Clear { buffer, bytes } => {
                    encoder.clear_buffer(
                        &buffer.inner.borrow().raw,
                        bytes.start,
                        Some(bytes.end - bytes.start),
                    );
                }
                ComputeCommand::Dispatch(dispatch) => {
                    if let ComputeWorkgroups::Direct(workgroups) = &dispatch.workgroups
                        && workgroups.contains(&0)
                    {
                        continue;
                    }
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Compute Dispatch"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&dispatch.pipeline.inner.raw);
                    for (index, group) in dispatch.bind_groups.iter().enumerate() {
                        pass.set_bind_group(index as u32, &group.inner.raw, &[]);
                        for binding in &group.inner.textures {
                            if binding.access.writable() {
                                written_textures
                                    .entry(binding.texture.id())
                                    .or_insert_with(|| binding.texture.clone());
                            }
                        }
                    }
                    match &dispatch.workgroups {
                        ComputeWorkgroups::Direct(workgroups) => {
                            pass.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2]);
                        }
                        ComputeWorkgroups::Indirect(arguments) => {
                            pass.dispatch_workgroups_indirect(
                                &arguments.as_ref().inner.borrow().raw,
                                0,
                            );
                        }
                    }
                }
            }
        }
        self.ctx.queue.submit(Some(encoder.finish()));
        for texture in written_textures.into_values() {
            texture.mark_written();
        }
        Ok(())
    }

    fn create_sampler(&mut self, desc: SamplerDescriptor) -> Result<Sampler, String> {
        log::trace!("Creating Sampler (label={:?})", desc.label);
        let raw = self.ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: desc.label,
            address_mode_u: desc.wrap_x.as_wgpu(),
            address_mode_v: desc.wrap_y.as_wgpu(),
            address_mode_w: desc.wrap_z.as_wgpu(),
            mag_filter: desc.mag_filter.as_wgpu(),
            min_filter: desc.min_filter.as_wgpu(),
            mipmap_filter: desc.mipmap_filter.as_wgpu_mipmap(),
            ..Default::default()
        });
        Ok(Sampler {
            id: resource_id(&mut self.next_resource_id),
            raw: Arc::new(raw),
            wrap_x: desc.wrap_x,
            wrap_y: desc.wrap_y,
            wrap_z: desc.wrap_z,
            mag_filter: desc.mag_filter,
            min_filter: desc.min_filter,
            mipmap_filter: desc.mipmap_filter,
        })
    }

    fn create_texture(
        &mut self,
        desc: TextureDescriptor,
        upload: TextureUpload,
    ) -> Result<Texture, String> {
        log::trace!("Creating Texture (label={:?})", desc.label);
        let generates_mipmaps = upload.generates_mipmaps();
        let id = resource_id(&mut self.next_resource_id);
        let texture = create_texture(id, &self.ctx, desc, upload)?;
        if generates_mipmaps {
            self.mipmap_generator.generate(
                &self.ctx.device,
                &self.ctx.queue,
                &texture.raw,
                texture.format,
                texture.mip_level_count,
            );
        }
        Ok(texture)
    }

    fn generate_mipmaps(&mut self, texture: &Texture) -> Result<(), String> {
        if !texture.is_writable() {
            return Err(format!("Texture '{:?}' is not writable", texture.id()));
        }
        if texture.mip_level_count == 1 {
            if texture.width() == 1.0 && texture.height() == 1.0 {
                return Ok(());
            }
            return Err(format!(
                "Texture '{:?}' has no allocated mip levels",
                texture.id()
            ));
        }
        MipmapGenerator::validate_format(
            &self.ctx.adapter,
            &self.ctx.device,
            None,
            texture.format,
        )?;
        self.mipmap_generator.generate(
            &self.ctx.device,
            &self.ctx.queue,
            &texture.raw,
            texture.format,
            texture.mip_level_count,
        );
        texture.mark_written();
        Ok(())
    }

    fn write_texture(
        &mut self,
        texture: &Texture,
        offset: UVec2,
        size: UVec2,
        data: &[u8],
    ) -> Result<(), String> {
        if !texture.write {
            return Err(format!("Texture '{:?}' is not writable", texture.id()));
        }
        let bytes_per_texel = texture.format.bytes_per_texel().ok_or_else(|| {
            format!(
                "Texture format {:?} does not support pixel writes",
                texture.format
            )
        })?;
        let bytes_per_row = size
            .x
            .checked_mul(bytes_per_texel)
            .ok_or_else(|| "Texture write row size overflows".to_string())?;
        let mut copy = texture.raw.as_image_copy();
        copy.origin = Origin3d {
            x: offset.x,
            y: offset.y,
            z: 0,
        };
        self.ctx.queue.write_texture(
            copy,
            data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(size.y),
            },
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
        );
        texture.mark_written();
        Ok(())
    }

    fn create_render_texture(
        &mut self,
        desc: RenderTextureDescriptor,
    ) -> Result<RenderTexture, String> {
        let mut next_resource_id = self.next_resource_id;
        let texture = self.prepare_render_texture(&mut next_resource_id, desc)?;
        self.next_resource_id = next_resource_id;
        Ok(texture)
    }

    fn limits(&self) -> Limits {
        let raw_limits = self.ctx.device.limits();
        let surface_formats = self
            .surface
            .capabilities
            .formats
            .iter()
            .filter_map(|tx| TextureFormat::from_wgpu(*tx))
            .collect();

        let (max_storage_binding_size, max_storage_buffers_per_shader_stage) =
            if self.supports_storage_buffers() {
                (
                    raw_limits.max_storage_buffer_binding_size,
                    raw_limits.max_storage_buffers_per_shader_stage,
                )
            } else {
                (0, 0)
            };

        let compute_supported = self.supports_compute();
        let (
            max_compute_workgroups_per_dimension,
            max_compute_workgroup_size,
            max_compute_invocations_per_workgroup,
        ) = if compute_supported {
            (
                raw_limits.max_compute_workgroups_per_dimension,
                [
                    raw_limits.max_compute_workgroup_size_x,
                    raw_limits.max_compute_workgroup_size_y,
                    raw_limits.max_compute_workgroup_size_z,
                ],
                raw_limits.max_compute_invocations_per_workgroup,
            )
        } else {
            (0, [0; 3], 0)
        };

        Limits {
            max_texture_size_2d: raw_limits.max_texture_dimension_2d,
            max_texture_size_3d: raw_limits.max_texture_dimension_3d,
            max_buffer_size: raw_limits.max_buffer_size,
            max_storage_binding_size,
            max_storage_buffers_per_shader_stage,
            compute_supported,
            max_compute_workgroups_per_dimension,
            max_compute_workgroup_size,
            max_compute_invocations_per_workgroup,
            surface_formats,
        }
    }

    fn stats(&self) -> GpuStats {
        self.last_frame_stats
    }
}

#[inline(always)]
fn resource_id<T: From<u64>>(count: &mut u64) -> T {
    let id = *count;
    *count += 1;
    T::from(id)
}

impl GfxBackend {
    fn resolve_shader(&mut self, input: ShaderInput<'_>) -> Result<crate::gfx::Shader, String> {
        match input {
            ShaderInput::Source(source) => self.create_shader(source),
            ShaderInput::Shader(shader) => Ok(shader.clone()),
        }
    }

    fn resolve_layouts(
        &mut self,
        label: Option<&str>,
        interface: &ShaderInterface,
        manual: &ArrayVec<BindGroupLayout, MAX_BIND_GROUPS_PER_PIPELINE>,
    ) -> Result<ArrayVec<BindGroupLayoutRef, MAX_BIND_GROUPS_PER_PIPELINE>, String> {
        let pipeline = label.unwrap_or("Pipeline");
        let reflected_last = interface.groups.last().map(|group| group.group as usize);
        let manual_last = manual.len().checked_sub(1);
        let Some(last) = reflected_last.into_iter().chain(manual_last).max() else {
            return Ok(ArrayVec::new());
        };
        if last >= MAX_BIND_GROUPS_PER_PIPELINE {
            return Err(format!(
                "{pipeline} uses bind group {last}, but RKit supports at most {MAX_BIND_GROUPS_PER_PIPELINE} groups"
            ));
        }

        let mut normalized: ArrayVec<
            ArrayVec<crate::gfx::BindingType, MAX_BINDING_ENTRIES>,
            MAX_BIND_GROUPS_PER_PIPELINE,
        > = ArrayVec::new();
        for group in 0..=last {
            let reflected = interface
                .groups
                .iter()
                .find(|candidate| candidate.group == group as u32);
            let entries = match manual.get(group) {
                Some(layout) => {
                    let mut entries = layout.entries.clone();
                    entries.sort_unstable_by_key(|entry| entry.location);
                    if let Some(reflected) = reflected {
                        for required in &reflected.bindings {
                            let declared = entries
                                .iter()
                                .find(|entry| entry.location == required.binding.location)
                                .ok_or_else(|| {
                                    format!(
                                        "{} requires @group({group}) @binding({})",
                                        pipeline, required.binding.location
                                    )
                                })?;
                            if !binding_type_covers(declared.typ, required.binding.typ) {
                                return Err(format!(
                                    "{} @group({group}) @binding({}) does not match the selected shader entry",
                                    pipeline, required.binding.location
                                ));
                            }
                            let covers_visibility = (!required.binding.visible_vertex
                                || declared.visible_vertex)
                                && (!required.binding.visible_fragment
                                    || declared.visible_fragment)
                                && (!required.binding.visible_compute || declared.visible_compute);
                            if !covers_visibility {
                                return Err(format!(
                                    "{} @group({group}) @binding({}) does not cover selected shader visibility",
                                    pipeline, required.binding.location
                                ));
                            }
                        }
                    }
                    entries
                }
                None => reflected.map_or_else(ArrayVec::new, |group| {
                    group
                        .bindings
                        .iter()
                        .map(|binding| binding.binding)
                        .collect()
                }),
            };
            normalized
                .try_push(entries)
                .map_err(|_| format!("{pipeline} uses too many bind groups"))?;
        }
        self.validate_layouts(&normalized)?;

        normalized
            .into_iter()
            .map(|entries| self.cached_layout(label, entries))
            .collect()
    }

    fn cached_layout(
        &mut self,
        label: Option<&str>,
        entries: ArrayVec<crate::gfx::BindingType, MAX_BINDING_ENTRIES>,
    ) -> Result<BindGroupLayoutRef, String> {
        let shape = LayoutShape { entries };
        if let Some(layout) = self.layout_cache.get(&shape) {
            return Ok(layout.clone());
        }

        let raw_entries = shape
            .entries
            .iter()
            .map(|entry| wgpu::BindGroupLayoutEntry {
                binding: entry.location,
                visibility: wgpu_shader_visibility(
                    entry.visible_vertex,
                    entry.visible_fragment,
                    entry.visible_compute,
                ),
                ty: match entry.typ {
                    BindType::Texture(sample_type) => wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu_sample_type(sample_type),
                    },
                    BindType::Sampler { filtering } => wgpu::BindingType::Sampler(if filtering {
                        wgpu::SamplerBindingType::Filtering
                    } else {
                        wgpu::SamplerBindingType::NonFiltering
                    }),
                    BindType::Uniform => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    BindType::StorageReadonly | BindType::StorageReadwrite => {
                        wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: matches!(entry.typ, BindType::StorageReadonly),
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        }
                    }
                    BindType::StorageTexture { format, access } => {
                        wgpu::BindingType::StorageTexture {
                            access: wgpu_storage_texture_access(access),
                            format: format.as_wgpu(),
                            view_dimension: wgpu::TextureViewDimension::D2,
                        }
                    }
                },
                count: None,
            })
            .collect::<Vec<_>>();
        let layout = BindGroupLayoutRef {
            id: resource_id(&mut self.next_resource_id),
            raw: Arc::new(self.ctx.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label,
                    entries: &raw_entries,
                },
            )),
            entries: shape.entries.clone(),
        };
        self.layout_cache.insert(shape, layout.clone());
        Ok(layout)
    }

    fn build_render_pipeline(
        &self,
        recipe: &PipelineRecipe,
        bind_group_layouts: &ArrayVec<BindGroupLayoutRef, MAX_BIND_GROUPS_PER_PIPELINE>,
    ) -> wgpu::RenderPipeline {
        let layout_refs = bind_group_layouts
            .iter()
            .map(|layout| Some(layout.raw.as_ref()))
            .collect::<Vec<_>>();
        let pipeline_layout =
            self.ctx
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: recipe.label.as_deref(),
                    bind_group_layouts: &layout_refs,
                    immediate_size: 0,
                });

        let (attributes, mut buffers): (Vec<_>, Vec<_>) = recipe
            .vertex_layout
            .iter()
            .map(|layout| {
                let mut offset = 0;
                let attributes = layout
                    .attributes
                    .iter()
                    .map(|attribute| {
                        let attribute = wgpu::VertexAttribute {
                            format: attribute.format.as_wgpu(),
                            offset,
                            shader_location: attribute.location,
                        };
                        offset += attribute.format.size();
                        attribute
                    })
                    .collect::<Vec<_>>();
                let buffer = wgpu::VertexBufferLayout {
                    array_stride: offset,
                    step_mode: layout.step_mode.as_wgpu(),
                    attributes: &[],
                };
                (attributes, buffer)
            })
            .unzip();
        for (index, buffer) in buffers.iter_mut().enumerate() {
            buffer.attributes = &attributes[index];
        }
        let buffers = buffers.into_iter().map(Some).collect::<Vec<_>>();

        self.ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: recipe.label.as_deref(),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &recipe.shader,
                    entry_point: Some(&recipe.vs_entry),
                    compilation_options: Default::default(),
                    buffers: &buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &recipe.shader,
                    entry_point: Some(&recipe.fs_entry),
                    compilation_options: Default::default(),
                    targets: recipe.targets.as_slice(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: recipe.primitive.as_wgpu(),
                    cull_mode: recipe.cull_mode.map(|mode| mode.as_wgpu()),
                    ..Default::default()
                },
                depth_stencil: wgpu_depth_stencil(recipe.depth, recipe.stencil),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
    }

    pub(crate) async fn init(
        source: SurfaceSource,
        vsync: bool,
        win_size: UVec2,
        pixelated: bool,
    ) -> Result<Self, String> {
        let is_zero = win_size.x == 0 || win_size.y == 0;
        if is_zero {
            return Err("Cannot initialize a surface with a zero size".to_string());
        }

        let backend_override = Backends::from_env().filter(|backends| !backends.is_empty());

        #[cfg(target_arch = "wasm32")]
        {
            init_wasm_gfx(source, vsync, win_size, pixelated, backend_override).await
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            init_native_gfx(source, vsync, win_size, pixelated, backend_override).await
        }
    }

    #[cfg(native_windowed)]
    pub(crate) fn update_surface(
        &mut self,
        source: SurfaceSource,
        win_size: UVec2,
    ) -> Result<(), String> {
        let size = if win_size.x == 0 || win_size.y == 0 {
            UVec2::new(self.surface.config.width, self.surface.config.height)
        } else {
            win_size
        };
        if size.x == 0 || size.y == 0 {
            return Err("Cannot replace a surface without a nonzero size".to_string());
        }

        let owner = SurfaceOwner::new(source, &self.ctx.instance)?;
        let candidate = SurfaceCandidate::replacement(&self.ctx, owner, size, &self.surface)?;
        let mut next_resource_id = self.next_resource_id;
        let depth_texture = create_surface_depth(
            resource_id(&mut next_resource_id),
            &self.ctx,
            self.depth_format,
            size,
        )?;
        let offscreen = self
            .offscreen
            .as_ref()
            .ok_or_else(|| "Invalid Offscreen surface".to_string())?
            .prepare_resize(self, size, &mut next_resource_id)?;

        let surface = candidate.configure(&self.ctx.device, depth_texture);

        self.surface = surface;
        if let Some(resized) = offscreen {
            self.offscreen.as_mut().unwrap().commit_resize(resized);
        }
        self.next_resource_id = next_resource_id;
        self.frame = None;
        Ok(())
    }

    fn validate_compute_dispatch(
        &self,
        dispatch: &crate::gfx::ComputeDispatch<'_>,
    ) -> Result<(), String> {
        match &dispatch.workgroups {
            ComputeWorkgroups::Direct(workgroups) => {
                if workgroups.contains(&0) {
                    return Ok(());
                }
                let maximum = self
                    .ctx
                    .device
                    .limits()
                    .max_compute_workgroups_per_dimension;
                if workgroups.iter().any(|count| *count > maximum) {
                    return Err(format!(
                        "Compute dispatch exceeds the device workgroup limit of {maximum} per dimension"
                    ));
                }
            }
            ComputeWorkgroups::Indirect(_) if !self.supports_indirect_execution() => {
                return Err(
                    "Indirect execution is unsupported by this graphics backend".to_string()
                );
            }
            ComputeWorkgroups::Indirect(_) => {}
        }
        if !self.supports_compute() {
            return Err("Compute dispatch is unsupported by this graphics backend".to_string());
        }
        validate_pipeline_bindings(
            "Compute pipeline",
            &dispatch.pipeline.inner.bind_group_layout,
            &dispatch.pipeline.inner.interface,
            &dispatch.bind_groups,
        )?;

        let mut buffers = HashMap::new();
        if let ComputeWorkgroups::Indirect(arguments) = &dispatch.workgroups {
            buffers.insert(arguments.id(), false);
        }
        let mut textures = HashMap::new();
        for group in &dispatch.bind_groups {
            for binding in &group.inner.buffers {
                let previous = buffers.insert(binding.buffer.id(), binding.writable);
                if let Some(previous) = previous
                    && (previous || binding.writable)
                {
                    return Err("Compute dispatch aliases a writable buffer binding".to_string());
                }
            }
            for binding in &group.inner.textures {
                let writable = binding.access.writable();
                let previous = textures.insert(binding.texture.id(), writable);
                if let Some(previous) = previous
                    && (previous || writable)
                {
                    return Err("Compute dispatch aliases a writable texture binding".to_string());
                }
            }
        }
        Ok(())
    }

    fn supports_compute(&self) -> bool {
        self.ctx.supports_compute
    }

    fn validate_workgroup_requirements(
        &self,
        label: Option<&str>,
        entry: &str,
        workgroup_size: [u32; 3],
        workgroup_storage_size: u64,
    ) -> Result<(), String> {
        let limits = self.ctx.device.limits();
        let invocations = workgroup_size.iter().try_fold(1u32, |total, size| {
            total
                .checked_mul(*size)
                .ok_or_else(|| "Compute workgroup size overflows".to_string())
        })?;
        if workgroup_size.iter().any(|size| *size == 0)
            || workgroup_size[0] > limits.max_compute_workgroup_size_x
            || workgroup_size[1] > limits.max_compute_workgroup_size_y
            || workgroup_size[2] > limits.max_compute_workgroup_size_z
            || invocations > limits.max_compute_invocations_per_workgroup
        {
            return Err(format!(
                "{} compute entry '{entry}' has unsupported @workgroup_size({}, {}, {})",
                label.unwrap_or("Compute pipeline"),
                workgroup_size[0],
                workgroup_size[1],
                workgroup_size[2],
            ));
        }
        let maximum = u64::from(limits.max_compute_workgroup_storage_size);
        if workgroup_storage_size > maximum {
            return Err(format!(
                "{} compute entry '{entry}' requires {workgroup_storage_size} bytes of workgroup storage, but the device supports {maximum}",
                label.unwrap_or("Compute pipeline")
            ));
        }
        Ok(())
    }

    fn supports_storage_buffers(&self) -> bool {
        let limits = self.ctx.device.limits();
        !matches!(self.ctx.adapter.get_info().backend, wgpu::Backend::Gl)
            && limits.max_storage_buffers_per_shader_stage > 0
            && limits.max_storage_buffer_binding_size > 0
    }

    fn supports_indirect_execution(&self) -> bool {
        self.ctx
            .adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::INDIRECT_EXECUTION)
    }

    fn validate_storage_visibility(&self, binding: &crate::gfx::BindingType) -> Result<(), String> {
        let is_storage_buffer = matches!(
            binding.typ,
            BindType::StorageReadonly | BindType::StorageReadwrite
        );
        let writable = matches!(
            binding.typ,
            BindType::StorageReadwrite
                | BindType::StorageTexture {
                    access: StorageTextureAccess::Writeonly | StorageTextureAccess::Readwrite,
                    ..
                }
        );
        if !binding.visible_vertex && !binding.visible_fragment && !binding.visible_compute {
            return Err(format!(
                "Storage binding {} is not visible to a shader stage",
                binding.location
            ));
        }

        let downlevel = self.ctx.adapter.get_downlevel_capabilities().flags;
        if binding.visible_vertex {
            if is_storage_buffer && !downlevel.contains(wgpu::DownlevelFlags::VERTEX_STORAGE) {
                return Err(
                    "Storage buffers in vertex shaders are unsupported by this graphics backend"
                        .to_string(),
                );
            }
            if writable
                && !self
                    .ctx
                    .device
                    .features()
                    .contains(wgpu::Features::VERTEX_WRITABLE_STORAGE)
            {
                return Err("Writable storage bindings in vertex shaders are unsupported by this graphics backend".to_string());
            }
        }
        if writable
            && binding.visible_fragment
            && !downlevel.contains(wgpu::DownlevelFlags::FRAGMENT_WRITABLE_STORAGE)
        {
            return Err("Writable storage bindings in fragment shaders are unsupported by this graphics backend".to_string());
        }
        Ok(())
    }

    fn validate_buffer_allocation(&self, usage: BufferUsage, size: usize) -> Result<(), String> {
        let limits = self.ctx.device.limits();
        let max_buffer_size = usize::try_from(limits.max_buffer_size).unwrap_or(usize::MAX);
        if size > max_buffer_size {
            return Err(format!(
                "Buffer allocation of {size} bytes exceeds the device maximum of {max_buffer_size} bytes"
            ));
        }

        if usage == BufferUsage::Storage {
            if size % 4 != 0 {
                return Err(
                    "Storage buffer allocations must have a size divisible by four bytes"
                        .to_string(),
                );
            }
            if !self.supports_storage_buffers() {
                return Err(
                    "Read-only storage buffers are unsupported by this graphics backend"
                        .to_string(),
                );
            }
            let max_storage_binding_size =
                usize::try_from(limits.max_storage_buffer_binding_size).unwrap_or(usize::MAX);
            if size > max_storage_binding_size {
                return Err(format!(
                    "Storage buffer allocation of {size} bytes exceeds the device binding maximum of {max_storage_binding_size} bytes"
                ));
            }
        }
        Ok(())
    }

    fn validate_layouts(
        &self,
        layouts: &[ArrayVec<crate::gfx::BindingType, MAX_BINDING_ENTRIES>],
    ) -> Result<(), String> {
        let mut storage_buffers = [0; 3];
        let mut storage_textures = [0; 3];
        for layout in layouts {
            for (index, entry) in layout.iter().enumerate() {
                if layout[..index]
                    .iter()
                    .any(|previous| previous.location == entry.location)
                {
                    return Err(format!(
                        "Bind group layout contains duplicate binding {}",
                        entry.location
                    ));
                }
                let visibility = [
                    entry.visible_vertex,
                    entry.visible_fragment,
                    entry.visible_compute,
                ];
                let counts = match entry.typ {
                    BindType::StorageReadonly | BindType::StorageReadwrite => {
                        self.validate_storage_visibility(entry)?;
                        if !self.supports_storage_buffers() {
                            return Err("Storage buffers are unsupported by this graphics backend"
                                .to_string());
                        }
                        &mut storage_buffers
                    }
                    BindType::StorageTexture { format, access } => {
                        self.validate_storage_visibility(entry)?;
                        validate_storage_texture_format(&self.ctx, None, format, access)?;
                        &mut storage_textures
                    }
                    _ => continue,
                };
                for (count, visible) in counts.iter_mut().zip(visibility) {
                    *count += usize::from(visible);
                }
            }
        }

        let limits = self.ctx.device.limits();
        for (resource, counts, maximum) in [
            (
                "storage buffers",
                storage_buffers,
                limits.max_storage_buffers_per_shader_stage as usize,
            ),
            (
                "storage textures",
                storage_textures,
                limits.max_storage_textures_per_shader_stage as usize,
            ),
        ] {
            for (stage, count) in ["vertex", "fragment", "compute"].into_iter().zip(counts) {
                if count > maximum {
                    return Err(format!(
                        "Pipeline requires {count} {resource} in the {stage} stage, but the device supports {maximum}"
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_bind_group_entries(
        &self,
        layout: &BindGroupLayoutRef,
        entries: &[BindGroupEntry<'_>],
    ) -> Result<(), String> {
        if entries.len() != layout.entries.len() {
            return Err(format!(
                "Bind group provides {} entries, but its layout requires {}",
                entries.len(),
                layout.entries.len()
            ));
        }

        for (index, entry) in entries.iter().enumerate() {
            let location = match entry {
                BindGroupEntry::Texture { location, .. }
                | BindGroupEntry::Sampler { location, .. }
                | BindGroupEntry::Uniform { location, .. }
                | BindGroupEntry::StorageReadonly { location, .. }
                | BindGroupEntry::StorageReadwrite { location, .. }
                | BindGroupEntry::StorageTexture { location, .. } => *location,
            };
            if entries[..index].iter().any(|previous| match previous {
                BindGroupEntry::Texture {
                    location: previous, ..
                }
                | BindGroupEntry::Sampler {
                    location: previous, ..
                }
                | BindGroupEntry::Uniform {
                    location: previous, ..
                }
                | BindGroupEntry::StorageReadonly {
                    location: previous, ..
                }
                | BindGroupEntry::StorageReadwrite {
                    location: previous, ..
                }
                | BindGroupEntry::StorageTexture {
                    location: previous, ..
                } => *previous == location,
            }) {
                return Err(format!("Bind group contains duplicate binding {location}"));
            }
            let expected = layout
                .entries
                .iter()
                .find(|expected| expected.location == location)
                .ok_or_else(|| format!("Bind group provides unexpected binding {location}"))?;
            let matches = match (expected.typ, entry) {
                (BindType::Texture(_), BindGroupEntry::Texture { .. })
                | (BindType::Sampler { .. }, BindGroupEntry::Sampler { .. })
                | (BindType::Uniform, BindGroupEntry::Uniform { .. })
                | (BindType::StorageReadonly, BindGroupEntry::StorageReadonly { .. })
                | (BindType::StorageReadwrite, BindGroupEntry::StorageReadwrite { .. }) => true,
                (
                    BindType::StorageTexture { access, .. },
                    BindGroupEntry::StorageTexture {
                        access: requested, ..
                    },
                ) => access == *requested,
                _ => false,
            };
            if !matches {
                return Err(format!(
                    "Bind group binding {location} does not match its layout"
                ));
            }
            match entry {
                BindGroupEntry::Uniform { buffer, .. }
                    if buffer.usage() != BufferUsage::Uniform =>
                {
                    return Err(format!(
                        "Bind group uniform binding {location} requires a uniform buffer"
                    ));
                }
                BindGroupEntry::StorageReadonly { buffer, .. }
                | BindGroupEntry::StorageReadwrite { buffer, .. }
                    if buffer.usage() != BufferUsage::Storage =>
                {
                    return Err(format!(
                        "Bind group storage binding {location} requires a storage buffer"
                    ));
                }
                BindGroupEntry::Sampler { sampler, .. } => {
                    let BindType::Sampler { filtering } = expected.typ else {
                        unreachable!();
                    };
                    if !filtering && sampler.is_filtering() {
                        return Err(format!(
                            "Bind group sampler binding {location} requires a nonfiltering sampler"
                        ));
                    }
                }
                BindGroupEntry::Texture { texture, .. } => {
                    let BindType::Texture(sample_type) = expected.typ else {
                        unreachable!();
                    };
                    validate_sampled_texture_format(&self.ctx, texture, sample_type)?;
                }
                BindGroupEntry::StorageTexture { texture, .. } => {
                    let BindType::StorageTexture { format, .. } = expected.typ else {
                        unreachable!();
                    };
                    if !texture.is_storage() || texture.format() != format {
                        return Err(format!(
                            "Bind group storage texture binding {location} requires a matching storage texture"
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) fn prepare_bind_group(
        &self,
        next_resource_id: &mut u64,
        desc: BindGroupDescriptor,
    ) -> Result<BindGroup, String> {
        log::trace!("Creating BindGroup (label={:?})", desc.label);
        let layout = desc
            .layout
            .ok_or("Cannot create binding group with a missing layout.")?;
        self.validate_bind_group_entries(layout, &desc.entry)?;
        let raw_buffers: ArrayVec<_, MAX_BINDING_ENTRIES> = desc
            .entry
            .iter()
            .map(|entry| match entry {
                BindGroupEntry::Uniform { buffer, .. }
                | BindGroupEntry::StorageReadonly { buffer, .. }
                | BindGroupEntry::StorageReadwrite { buffer, .. } => {
                    Some(buffer.inner.borrow().raw.clone())
                }
                _ => None,
            })
            .collect();
        let entries: ArrayVec<_, MAX_BINDING_ENTRIES> = desc
            .entry
            .iter()
            .enumerate()
            .map(|(idx, entry)| match entry {
                BindGroupEntry::Texture { location, texture }
                | BindGroupEntry::StorageTexture {
                    location, texture, ..
                } => wgpu::BindGroupEntry {
                    binding: *location,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                BindGroupEntry::Uniform { location, .. }
                | BindGroupEntry::StorageReadonly { location, .. }
                | BindGroupEntry::StorageReadwrite { location, .. } => wgpu::BindGroupEntry {
                    binding: *location,
                    resource: raw_buffers[idx].as_ref().unwrap().as_entire_binding(),
                },
                BindGroupEntry::Sampler { location, sampler } => wgpu::BindGroupEntry {
                    binding: *location,
                    resource: wgpu::BindingResource::Sampler(&sampler.raw),
                },
            })
            .collect();
        let buffer_bindings = desc
            .entry
            .iter()
            .filter_map(|entry| match entry {
                BindGroupEntry::Uniform { location, buffer }
                | BindGroupEntry::StorageReadonly { location, buffer } => Some(BufferBinding {
                    location: *location,
                    size: buffer.size() as u64,
                    buffer: (*buffer).clone(),
                    writable: false,
                }),
                BindGroupEntry::StorageReadwrite { location, buffer } => Some(BufferBinding {
                    location: *location,
                    size: buffer.size() as u64,
                    buffer: (*buffer).clone(),
                    writable: true,
                }),
                _ => None,
            })
            .collect();
        let textures = desc
            .entry
            .iter()
            .filter_map(|entry| match entry {
                BindGroupEntry::Texture { texture, .. } => {
                    Some(crate::backend::wgpu::TextureBinding {
                        texture: (*texture).clone(),
                        access: TextureBindingAccess::Sampled,
                    })
                }
                BindGroupEntry::StorageTexture {
                    texture, access, ..
                } => Some(crate::backend::wgpu::TextureBinding {
                    texture: (*texture).clone(),
                    access: TextureBindingAccess::Storage(*access),
                }),
                _ => None,
            })
            .collect();
        let raw = self
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: desc.label,
                layout: &layout.raw,
                entries: &entries,
            });

        Ok(BindGroup {
            inner: Arc::new(BindGroupInner {
                id: resource_id(next_resource_id),
                layout: layout.id,
                raw,
                buffers: buffer_bindings,
                textures,
            }),
        })
    }

    pub(crate) fn prepare_render_texture(
        &self,
        next_resource_id: &mut u64,
        desc: RenderTextureDescriptor,
    ) -> Result<RenderTexture, String> {
        log::trace!("Creating RenderTexture (label={:?})", desc.label);
        let format = match desc.format {
            Some(format) => format,
            None => TextureFormat::from_wgpu(self.surface.config.format)
                .ok_or_else(|| "Unsupported surface texture format".to_string())?,
        };
        let color_label = format!("RenderTexture (label={:?}) inner color texture", desc.label);
        let texture = create_texture(
            resource_id(next_resource_id),
            &self.ctx,
            TextureDescriptor {
                label: Some(&color_label),
                format,
                write: true,
                storage: false,
            },
            if desc.mipmaps {
                TextureUpload::Generate(TextureMipLevel::new(&[], desc.width, desc.height))
            } else {
                TextureUpload::Single(TextureMipLevel::new(&[], desc.width, desc.height))
            },
        )?;
        let color_attachment = Arc::new(texture.raw.create_view(&wgpu::TextureViewDescriptor {
            label: Some("RenderTexture level-0 attachment"),
            format: Some(format.as_wgpu()),
            base_mip_level: 0,
            mip_level_count: Some(1),
            ..Default::default()
        }));
        let depth_texture = if desc.depth {
            let depth_label = format!("RenderTexture (label={:?}) inner depth texture", desc.label);
            Some(create_texture(
                resource_id(next_resource_id),
                &self.ctx,
                TextureDescriptor {
                    label: Some(&depth_label),
                    format: SURFACE_DEFAULT_DEPTH_FORMAT,
                    write: true,
                    storage: false,
                },
                TextureUpload::Single(TextureMipLevel::new(&[], desc.width, desc.height)),
            )?)
        } else {
            None
        };

        Ok(RenderTexture {
            id: resource_id(next_resource_id),
            texture,
            color_attachment,
            depth_texture,
        })
    }

    async fn new(
        source: SurfaceSource,
        vsync: bool,
        win_size: UVec2,
        pixelated: bool,
        backends: Backends,
    ) -> Result<Self, String> {
        let depth_format = SURFACE_DEFAULT_DEPTH_FORMAT; // make it configurable?
        let mut next_resource_id = 0;

        #[cfg(not(target_arch = "wasm32"))]
        let mut descriptor =
            InstanceDescriptor::new_with_display_handle(Box::new(source.display.clone()));
        #[cfg(target_arch = "wasm32")]
        let mut descriptor = InstanceDescriptor::new_with_display_handle(Box::new(WebDisplay));
        descriptor.backends = backends;
        descriptor.flags = InstanceFlags::from_env_or_default();
        descriptor.backend_options = BackendOptions {
            gl: GlBackendOptions::from_env_or_default(),
            dx12: Dx12BackendOptions {
                shader_compiler: wgpu::Dx12Compiler::StaticDxc,
                ..Dx12BackendOptions::from_env_or_default()
            },
            ..BackendOptions::from_env_or_default()
        };

        // this will automatically fallback to webgl if webgpu is not supported
        let instance = new_instance_with_webgpu_detection(descriptor).await;

        let (ctx, surface) = {
            let owner = SurfaceOwner::new(source, &instance)?;
            let ctx = Context::new(instance, owner.raw()).await?;
            let candidate = SurfaceCandidate::initial(&ctx, owner, win_size, vsync)?;
            let depth_texture = create_surface_depth(
                resource_id(&mut next_resource_id),
                &ctx,
                depth_format,
                win_size,
            )?;
            let surface = candidate.configure(&ctx.device, depth_texture);
            (ctx, surface)
        };

        let mipmap_generator = MipmapGenerator::new(&ctx.device);
        let mut bck = Self {
            next_resource_id,
            layout_cache: HashMap::new(),
            ctx,
            #[cfg(native_windowed)]
            depth_format,
            surface,
            frame: None,
            offscreen: None,
            mipmap_generator,
            readbacks: ReadbackManager::default(),
            last_frame_stats: GpuStats::default(),
            current_stats: GpuStats::default(),
        };

        let offscreen = OffscreenSurfaceData::new(&mut bck, pixelated)?;
        bck.offscreen = Some(offscreen);

        Ok(bck)
    }

    fn push_frame(&mut self) -> Result<(), String> {
        let (frame, reconfigure_after_frame) = match self.surface.frame() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                let config = &self.surface.config;
                let size = UVec2::new(config.width, config.height);
                self.try_resize(size.x, size.y)?;
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.recreate(&self.ctx).map_err(|e| {
                    format!(
                        "Cannot recover lost WGPU surface while preserving the existing adapter, device, and color format: {e}"
                    )
                })?;
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("WGPU surface texture acquisition failed validation".to_string());
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Frame Encode"),
            });
        self.frame = Some(DrawFrame {
            frame,
            view,
            encoder,
            dirty: false,
            reconfigure_after_frame,
        });

        Ok(())
    }

    pub(crate) fn render_to_frame(
        &mut self,
        frame: &mut DrawFrame,
        renderer: &Renderer,
    ) -> Result<(), String> {
        for pass in &renderer.passes {
            validate_render_resource_aliases(pass)?;
            let (uses_depth, uses_stencil) = pass_uses_depth_stencil(pass)?;
            let color = Some(wgpu::RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: pass
                        .clear_options
                        .color
                        .map_or(wgpu::LoadOp::Load, |color| {
                            wgpu::LoadOp::Clear(color.as_wgpu())
                        }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            });
            let depth = uses_depth.then(|| wgpu::Operations {
                load: pass
                    .clear_options
                    .depth
                    .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                store: StoreOp::Store,
            });
            let stencil = uses_stencil.then(|| wgpu::Operations {
                load: pass
                    .clear_options
                    .stencil
                    .map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                store: StoreOp::Store,
            });

            let mut raw_pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[color],
                    depth_stencil_attachment: (depth.is_some() || stencil.is_some()).then_some(
                        wgpu::RenderPassDepthStencilAttachment {
                            view: &self.surface.depth_texture.view,
                            depth_ops: depth,
                            stencil_ops: stencil,
                        },
                    ),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
            encode_render_commands(
                &mut raw_pass,
                &pass.commands,
                self.surface.config.width,
                self.surface.config.height,
            )?;
        }
        Ok(())
    }

    fn present_to_screen(&mut self) {
        let Some(mut df) = self.frame.take() else {
            return;
        };
        let reconfigure_after_frame = df.reconfigure_after_frame;

        if df.dirty {
            // TODO change this: "take" is ugly as hell
            let offscreen = self.offscreen.take().unwrap();
            offscreen.present(self, &mut df).unwrap();
            self.offscreen = Some(offscreen);

            let DrawFrame {
                frame,
                view,
                encoder,
                ..
            } = df;
            self.ctx.queue.submit(Some(encoder.finish()));
            self.current_stats.draw_calls += 1;
            drop(view);
            self.ctx.queue.present(frame);
        } else {
            drop(df);
        }

        if reconfigure_after_frame {
            self.surface.reconfigure(&self.ctx.device);
        }
    }

    #[inline]
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if let Err(err) = self.try_resize(width, height) {
            log::error!("Error resizing Gfx backend: {err}");
        }
    }

    fn try_resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Ok(());
        }

        let size = UVec2::new(width, height);
        let mut next_resource_id = self.next_resource_id;
        let resized = self
            .offscreen
            .as_ref()
            .ok_or_else(|| "Invalid Offscreen surface".to_string())?
            .prepare_resize(self, size, &mut next_resource_id)?;

        self.surface.configure_size(&self.ctx.device, size);
        if let Some(resized) = resized {
            self.offscreen.as_mut().unwrap().commit_resize(resized);
        }
        self.next_resource_id = next_resource_id;
        Ok(())
    }
}

fn create_surface_depth(
    id: TextureId,
    ctx: &Context,
    depth_format: TextureFormat,
    size: UVec2,
) -> Result<Texture, String> {
    create_texture(
        id,
        ctx,
        TextureDescriptor {
            label: Some("Depth Texture for Surface"),
            format: depth_format,
            write: true,
            storage: false,
        },
        TextureUpload::Single(TextureMipLevel::new(&[], size.x, size.y)),
    )
}

fn create_texture(
    id: TextureId,
    ctx: &Context,
    desc: TextureDescriptor,
    upload: TextureUpload,
) -> Result<Texture, String> {
    let Some(base) = upload.base() else {
        let label = desc
            .label
            .map_or(String::new(), |label| format!(" '{label}'"));
        return Err(format!("Texture{label} mipmap chain cannot be empty"));
    };
    if base.width == 0 || base.height == 0 {
        return Err(format!(
            "Texture {:?} dimensions must be nonzero",
            desc.label
        ));
    }
    validate_texture_size(ctx, desc.label, base.width, base.height)?;
    let generates_mipmaps = upload.generates_mipmaps();
    if desc.storage {
        validate_storage_texture_allocation(desc.label, desc.format, upload.mip_level_count())?;
    }
    let usage = texture_usage(desc.format, desc.write, generates_mipmaps, desc.storage);
    if generates_mipmaps {
        MipmapGenerator::validate_format(&ctx.adapter, &ctx.device, desc.label, desc.format)?;
    } else {
        validate_texture_format(ctx, desc.label, desc.format, usage)?;
    }

    let size = wgpu::Extent3d {
        width: base.width,
        height: base.height,
        depth_or_array_layers: 1,
    };
    let mip_level_count = upload.mip_level_count();
    let is_depth_texture = desc.format.is_depth();

    let view_formats = desc.format.view_formats();
    let raw = ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: desc.label,
        size,
        mip_level_count,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: desc.format.as_wgpu(),
        usage,
        view_formats: if ctx.supports_view_formats {
            &view_formats
        } else {
            &[]
        },
    });

    if !is_depth_texture {
        match upload {
            TextureUpload::Single(level) | TextureUpload::Generate(level) => {
                write_texture_level(&ctx.queue, &raw, desc.format, level, 0)?;
            }
            TextureUpload::Levels(levels) => {
                for (mip_level, level) in levels.iter().copied().enumerate() {
                    write_texture_level(&ctx.queue, &raw, desc.format, level, mip_level as u32)?;
                }
            }
        }
    }

    let view = raw.create_view(&wgpu::TextureViewDescriptor {
        format: Some(desc.format.as_wgpu()),
        ..Default::default()
    });

    Ok(Texture {
        id,
        raw: Arc::new(raw),
        view: Arc::new(view),
        size: vec2(size.width as _, size.height as _),
        write: desc.write,
        storage: desc.storage,
        copyable: usage.contains(wgpu::TextureUsages::COPY_SRC),
        format: desc.format,
        mip_level_count,
        revision: Arc::new(AtomicU32::new(0)),
    })
}

fn write_texture_level(
    queue: &Queue,
    texture: &wgpu::Texture,
    format: TextureFormat,
    level: TextureMipLevel<'_>,
    mip_level: u32,
) -> Result<(), String> {
    if level.bytes.is_empty() {
        return Ok(());
    }
    let bytes_per_texel = format
        .bytes_per_texel()
        .ok_or_else(|| format!("Texture format {format:?} does not support color uploads"))?;
    let bytes_per_row = level
        .width
        .checked_mul(bytes_per_texel)
        .ok_or_else(|| "Texture upload row size overflows".to_string())?;
    let mut copy = texture.as_image_copy();
    copy.mip_level = mip_level;
    queue.write_texture(
        copy,
        level.bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(bytes_per_row),
            rows_per_image: Some(level.height),
        },
        wgpu::Extent3d {
            width: level.width,
            height: level.height,
            depth_or_array_layers: 1,
        },
    );
    Ok(())
}

fn texture_usage(
    format: TextureFormat,
    writable: bool,
    generates_mipmaps: bool,
    storage: bool,
) -> wgpu::TextureUsages {
    let mut usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    if format.bytes_per_texel().is_some() {
        usage |= wgpu::TextureUsages::COPY_SRC;
    }
    if storage {
        usage |= wgpu::TextureUsages::STORAGE_BINDING;
    }
    if format.is_depth() || writable || generates_mipmaps {
        usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    }
    usage
}

fn validate_texture_size(
    ctx: &Context,
    label: Option<&str>,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let limit = ctx.device.limits().max_texture_dimension_2d;
    if width <= limit && height <= limit {
        return Ok(());
    }
    let label = label.map_or(String::new(), |label| format!(" '{label}'"));
    Err(format!(
        "Texture{label} size {width}x{height} exceeds the device maximum of {limit}x{limit}"
    ))
}

fn validate_storage_texture_allocation(
    label: Option<&str>,
    format: TextureFormat,
    mip_level_count: u32,
) -> Result<(), String> {
    if format.is_depth() || format.is_srgb() || format.bytes_per_texel().is_none() {
        return Err(texture_format_error(
            label,
            format,
            "is not a color format supported for storage textures",
        ));
    }
    if mip_level_count != 1 {
        return Err(texture_format_error(
            label,
            format,
            "storage textures must have exactly one mip level",
        ));
    }
    Ok(())
}

fn effective_texture_format(
    ctx: &Context,
    label: Option<&str>,
    format: TextureFormat,
) -> Result<(wgpu::TextureFormatFeatures, wgpu::TextureFormatFeatures), String> {
    let raw_format = format.as_wgpu();
    if !ctx
        .device
        .features()
        .contains(raw_format.required_features())
    {
        return Err(texture_format_error(
            label,
            format,
            "is not enabled on the active device",
        ));
    }
    Ok((
        ctx.adapter.get_texture_format_features(raw_format),
        raw_format.guaranteed_format_features(ctx.device.features()),
    ))
}

fn validate_sampled_texture_format(
    ctx: &Context,
    texture: &Texture,
    sample_type: SampledTextureType,
) -> Result<(), String> {
    if matches!(sample_type, SampledTextureType::Float { filterable: true })
        && matches!(
            texture.format(),
            TextureFormat::R32Float | TextureFormat::Rg32Float
        )
        && !ctx.supports_float32_filtering
    {
        return Err(format!(
            "Texture '{:?}' format {:?} requires FLOAT32_FILTERABLE for a filtering sampled binding",
            texture.id(),
            texture.format()
        ));
    }
    let actual = texture
        .format()
        .as_wgpu()
        .sample_type(None, Some(ctx.device.features()))
        .ok_or_else(|| {
            texture_format_error(
                None,
                texture.format(),
                "cannot be used as a sampled texture",
            )
        })?;
    let compatible = match (sample_type, actual) {
        (
            SampledTextureType::Float { filterable: false },
            wgpu::TextureSampleType::Float { .. },
        ) => true,
        (expected, actual) => wgpu_sample_type(expected) == actual,
    };
    if compatible {
        return Ok(());
    }
    Err(format!(
        "Texture '{:?}' format {:?} is incompatible with its sampled shader binding",
        texture.id(),
        texture.format()
    ))
}

fn validate_storage_texture_format(
    ctx: &Context,
    label: Option<&str>,
    format: TextureFormat,
    access: StorageTextureAccess,
) -> Result<(), String> {
    let (adapter_features, guaranteed_features) = effective_texture_format(ctx, label, format)?;
    if !adapter_features
        .allowed_usages
        .contains(wgpu::TextureUsages::STORAGE_BINDING)
        || !guaranteed_features
            .allowed_usages
            .contains(wgpu::TextureUsages::STORAGE_BINDING)
    {
        return Err(texture_format_error(
            label,
            format,
            "does not support storage bindings",
        ));
    }

    let access_flag = match access {
        StorageTextureAccess::Readonly => wgpu::TextureFormatFeatureFlags::STORAGE_READ_ONLY,
        StorageTextureAccess::Writeonly => wgpu::TextureFormatFeatureFlags::STORAGE_WRITE_ONLY,
        StorageTextureAccess::Readwrite => wgpu::TextureFormatFeatureFlags::STORAGE_READ_WRITE,
    };
    if adapter_features.flags.contains(access_flag)
        && guaranteed_features.flags.contains(access_flag)
    {
        return Ok(());
    }
    Err(texture_format_error(
        label,
        format,
        match access {
            StorageTextureAccess::Readonly => "does not support read-only storage bindings",
            StorageTextureAccess::Writeonly => "does not support write-only storage bindings",
            StorageTextureAccess::Readwrite => "does not support read-write storage bindings",
        },
    ))
}

fn texture_format_error(label: Option<&str>, format: TextureFormat, message: &str) -> String {
    let label = label.map_or(String::new(), |label| format!(" '{label}'"));
    format!("Texture{label} format {format:?} {message}")
}

fn validate_texture_format(
    ctx: &Context,
    label: Option<&str>,
    format: TextureFormat,
    usage: wgpu::TextureUsages,
) -> Result<(), String> {
    let (adapter_features, guaranteed_features) = effective_texture_format(ctx, label, format)?;
    if adapter_features.allowed_usages.contains(usage)
        && guaranteed_features.allowed_usages.contains(usage)
    {
        return Ok(());
    }
    let mut operations = vec!["sampling"];
    if usage.contains(wgpu::TextureUsages::COPY_DST) {
        operations.push("uploads");
    }
    if usage.contains(wgpu::TextureUsages::COPY_SRC) {
        operations.push("readback");
    }
    if usage.contains(wgpu::TextureUsages::STORAGE_BINDING) {
        operations.push("storage bindings");
    }
    if usage.contains(wgpu::TextureUsages::RENDER_ATTACHMENT) {
        operations.push("rendering");
    }
    Err(texture_format_error(
        label,
        format,
        &format!("does not support {}", operations.join(", ")),
    ))
}

impl Color {
    pub(crate) fn as_wgpu(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }
}

fn next_buffer_size(current: usize, required: usize) -> usize {
    current.max(required).next_power_of_two()
}
