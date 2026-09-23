use crate::{
    backend::wgpu::pipeline::{BindingRequirement, ShaderGroup, ShaderInterface},
    gfx::{
        BindType, BindingType, MAX_BINDING_ENTRIES, SampledTextureType, Shader,
        StorageTextureAccess, TextureFormat,
    },
};
use arrayvec::ArrayVec;
use std::{collections::BTreeMap, sync::Arc};
use wgpu::naga::{
    AddressSpace, GlobalVariable, Handle, ImageClass, ImageDimension, Module, ScalarKind,
    ShaderStage, StorageAccess, TypeInner,
    valid::{Capabilities, FunctionInfo, ValidationFlags, Validator},
};

use crate::gfx::consts::MAX_BIND_GROUPS_PER_PIPELINE;

pub(crate) fn create_shader(device: &wgpu::Device, source: &str) -> Result<Shader, String> {
    let module = wgpu::naga::front::wgsl::parse_str(source).map_err(|error| {
        format!(
            "Invalid WGSL shader source:\n{}",
            error.emit_to_string(source)
        )
    })?;
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .map_err(|error| {
            format!(
                "Invalid WGSL shader source:\n{}",
                error.emit_to_string(source)
            )
        })?;
    let raw = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });

    Ok(Shader {
        raw: Arc::new(raw),
        module: Arc::new(module),
        info: Arc::new(info),
    })
}

pub(crate) fn resolve_render(
    shader: &Shader,
    label: Option<&str>,
    vertex: &str,
    fragment: &str,
) -> Result<ShaderInterface, String> {
    let mut bindings = BTreeMap::new();
    add_entry_bindings(shader, label, vertex, ShaderStage::Vertex, &mut bindings)?;
    add_entry_bindings(
        shader,
        label,
        fragment,
        ShaderStage::Fragment,
        &mut bindings,
    )?;
    build_interface(bindings)
}

pub(crate) fn resolve_compute(
    shader: &Shader,
    label: Option<&str>,
    entry: &str,
) -> Result<(ShaderInterface, [u32; 3], u64), String> {
    let mut bindings = BTreeMap::new();
    let entry_index =
        add_entry_bindings(shader, label, entry, ShaderStage::Compute, &mut bindings)?;
    let entry_point = &shader.module.entry_points[entry_index];
    let entry_info = shader.info.get_entry_point(entry_index);
    let workgroup_storage_size = shader
        .module
        .global_variables
        .iter()
        .filter(|(handle, global)| {
            global.space == AddressSpace::WorkGroup && !entry_info[*handle].is_empty()
        })
        .try_fold(0u64, |total, (_, global)| {
            total
                .checked_add(
                    shader.module.types[global.ty]
                        .inner
                        .size(shader.module.to_ctx()) as u64,
                )
                .ok_or_else(|| {
                    format!(
                        "{} compute entry '{entry}' workgroup storage size overflows",
                        label.unwrap_or("Compute pipeline")
                    )
                })
        })?;
    Ok((
        build_interface(bindings)?,
        entry_point.workgroup_size,
        workgroup_storage_size,
    ))
}

fn build_interface(
    bindings: BTreeMap<(u32, u32), BindingRequirement>,
) -> Result<ShaderInterface, String> {
    let mut groups: ArrayVec<ShaderGroup, MAX_BIND_GROUPS_PER_PIPELINE> = ArrayVec::new();
    for ((group, _), binding) in bindings {
        match groups.last_mut() {
            Some(current) if current.group == group => current.bindings.try_push(binding).map_err(
                |_| {
                    format!(
                        "Selected shader entries use more than {MAX_BINDING_ENTRIES} bindings in @group({group})"
                    )
                },
            )?,
            _ => {
                let mut bindings = ArrayVec::new();
                bindings.try_push(binding).map_err(|_| {
                    format!(
                        "Selected shader entries use more than {MAX_BINDING_ENTRIES} bindings in @group({group})"
                    )
                })?;
                groups
                    .try_push(ShaderGroup { group, bindings })
                    .map_err(|_| "Shader uses more bind groups than RKit supports".to_string())?;
            }
        }
    }
    Ok(ShaderInterface { groups })
}

fn add_entry_bindings(
    shader: &Shader,
    label: Option<&str>,
    name: &str,
    stage: ShaderStage,
    bindings: &mut BTreeMap<(u32, u32), BindingRequirement>,
) -> Result<usize, String> {
    let index = shader
        .module
        .entry_points
        .iter()
        .position(|entry| entry.name == name && entry.stage == stage)
        .ok_or_else(|| missing_entry_error(label, name, stage, &shader.module))?;
    let entry = shader.info.get_entry_point(index);

    for (handle, global) in shader.module.global_variables.iter() {
        if entry[handle].is_empty() {
            continue;
        }
        let Some((typ, min_buffer_size)) = classify_binding(handle, global, &shader.module, entry)
            .map_err(|error| {
                let binding = global.binding.map_or_else(
                    || "without @group/@binding".to_string(),
                    |binding| format!("@group({}) @binding({})", binding.group, binding.binding),
                );
                format!(
                    "{} entry '{name}' {binding}: {error}",
                    stage_name(stage, label)
                )
            })?
        else {
            continue;
        };
        let binding = global.binding.ok_or_else(|| {
            format!(
                "{} entry '{name}' uses a bindable global without @group/@binding",
                stage_name(stage, label)
            )
        })?;
        let requirement = BindingRequirement {
            binding: BindingType {
                location: binding.binding,
                typ,
                visible_fragment: stage == ShaderStage::Fragment,
                visible_vertex: stage == ShaderStage::Vertex,
                visible_compute: stage == ShaderStage::Compute,
            },
            min_buffer_size,
        };
        let key = (binding.group, binding.binding);
        match bindings.get_mut(&key) {
            Some(existing) => {
                if !merge_binding_type(&mut existing.binding.typ, requirement.binding.typ) {
                    return Err(format!(
                        "{} selected entries disagree on @group({}) @binding({})",
                        label.unwrap_or("Render pipeline"),
                        binding.group,
                        binding.binding
                    ));
                }
                existing.binding.visible_vertex |= requirement.binding.visible_vertex;
                existing.binding.visible_fragment |= requirement.binding.visible_fragment;
                existing.min_buffer_size =
                    match (existing.min_buffer_size, requirement.min_buffer_size) {
                        (Some(left), Some(right)) => Some(left.max(right)),
                        (left, right) => left.or(right),
                    };
            }
            None => {
                bindings.insert(key, requirement);
            }
        }
    }
    Ok(index)
}

fn classify_binding(
    handle: Handle<GlobalVariable>,
    global: &GlobalVariable,
    module: &Module,
    entry: &FunctionInfo,
) -> Result<Option<(BindType, Option<u64>)>, String> {
    let inner = &module.types[global.ty].inner;
    let reflected = match global.space {
        AddressSpace::Function | AddressSpace::Private | AddressSpace::WorkGroup => {
            return Ok(None);
        }
        AddressSpace::Uniform => (BindType::Uniform, Some(inner.size(module.to_ctx()) as u64)),
        AddressSpace::Storage { access } if access == StorageAccess::LOAD => (
            BindType::StorageReadonly,
            Some(inner.size(module.to_ctx()) as u64),
        ),
        AddressSpace::Storage { .. } => (
            BindType::StorageReadwrite,
            Some(inner.size(module.to_ctx()) as u64),
        ),
        AddressSpace::Handle => match inner {
            TypeInner::Image {
                dim: ImageDimension::D2,
                arrayed: false,
                class: ImageClass::Sampled { kind, multi: false },
            } => (
                BindType::Texture(sampled_texture_type(
                    *kind,
                    entry.sampling_set.iter().any(|pair| pair.image == handle),
                )?),
                None,
            ),
            TypeInner::Image {
                dim: ImageDimension::D2,
                arrayed: false,
                class: ImageClass::Storage { format, access },
            } => (
                BindType::StorageTexture {
                    format: TextureFormat::from_naga_storage(*format)
                        .ok_or_else(|| "uses an unsupported storage texture format".to_string())?,
                    access: storage_texture_access(*access)?,
                },
                None,
            ),
            TypeInner::Sampler { comparison: false } => (
                BindType::Sampler {
                    filtering: sampler_filtering(handle, module, entry)?,
                },
                None,
            ),
            _ => return Err("uses an unsupported resource type".to_string()),
        },
        _ => return Err("uses an unsupported resource address space".to_string()),
    };
    Ok(Some(reflected))
}

fn merge_binding_type(existing: &mut BindType, next: BindType) -> bool {
    match (existing, next) {
        (
            BindType::Texture(SampledTextureType::Float { filterable }),
            BindType::Texture(SampledTextureType::Float {
                filterable: next_filterable,
            }),
        ) => {
            *filterable |= next_filterable;
            true
        }
        (
            BindType::Sampler { filtering },
            BindType::Sampler {
                filtering: next_filtering,
            },
        ) => *filtering == next_filtering,
        (current, next) => *current == next,
    }
}

fn sampler_filtering(
    sampler: Handle<GlobalVariable>,
    module: &Module,
    entry: &FunctionInfo,
) -> Result<bool, String> {
    let mut filtering = None;
    for image in entry
        .sampling_set
        .iter()
        .filter_map(|pair| (pair.sampler == sampler).then_some(pair.image))
    {
        let image = &module.global_variables[image];
        let TypeInner::Image {
            class: ImageClass::Sampled { kind, .. },
            ..
        } = &module.types[image.ty].inner
        else {
            return Err("uses a sampler with an unsupported texture type".to_string());
        };
        let required = match kind {
            ScalarKind::Float => true,
            ScalarKind::Sint | ScalarKind::Uint => false,
            _ => return Err("uses an unsupported sampled texture scalar type".to_string()),
        };
        if filtering.is_some_and(|current| current != required) {
            return Err("uses one sampler with incompatible texture sample classes".to_string());
        }
        filtering = Some(required);
    }
    Ok(filtering.unwrap_or(false))
}

fn sampled_texture_type(kind: ScalarKind, sampled: bool) -> Result<SampledTextureType, String> {
    match kind {
        ScalarKind::Float => Ok(SampledTextureType::Float {
            filterable: sampled,
        }),
        ScalarKind::Sint => Ok(SampledTextureType::Sint),
        ScalarKind::Uint => Ok(SampledTextureType::Uint),
        _ => Err("uses an unsupported sampled texture scalar type".to_string()),
    }
}

fn storage_texture_access(access: StorageAccess) -> Result<StorageTextureAccess, String> {
    match access {
        StorageAccess::LOAD => Ok(StorageTextureAccess::Readonly),
        StorageAccess::STORE => Ok(StorageTextureAccess::Writeonly),
        access if access == StorageAccess::LOAD | StorageAccess::STORE => {
            Ok(StorageTextureAccess::Readwrite)
        }
        _ => Err("uses an unsupported storage texture access mode".to_string()),
    }
}

fn missing_entry_error(
    label: Option<&str>,
    name: &str,
    stage: ShaderStage,
    module: &Module,
) -> String {
    let available = module
        .entry_points
        .iter()
        .filter(|entry| entry.stage == stage)
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    let pipeline = label.unwrap_or(match stage {
        ShaderStage::Compute => "Compute pipeline",
        _ => "Render pipeline",
    });
    format!(
        "{pipeline} has no {} entry '{name}' (available: {})",
        stage_name(stage, None),
        if available.is_empty() {
            "none".to_string()
        } else {
            available.join(", ")
        }
    )
}

fn stage_name(stage: ShaderStage, label: Option<&str>) -> String {
    let name = match stage {
        ShaderStage::Vertex => "vertex",
        ShaderStage::Fragment => "fragment",
        ShaderStage::Compute => "compute",
        _ => "unsupported",
    };
    label.map_or_else(|| name.to_string(), |label| format!("{label} {name}"))
}
