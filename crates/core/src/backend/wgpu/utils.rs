use crate::gfx::consts::SURFACE_DEFAULT_DEPTH_FORMAT;
use crate::gfx::{BlendComponent, BlendFactor, BlendMode, BlendOperation, DepthStencil, Stencil};
use wgpu::CompareFunction;

pub fn wgpu_shader_visibility(vertex: bool, fragment: bool, compute: bool) -> wgpu::ShaderStages {
    let mut v = wgpu::ShaderStages::NONE;
    if vertex {
        v |= wgpu::ShaderStages::VERTEX;
    }

    if fragment {
        v |= wgpu::ShaderStages::FRAGMENT;
    }

    if compute {
        v |= wgpu::ShaderStages::COMPUTE;
    }

    v
}

impl BlendMode {
    pub(crate) fn to_wgpu(&self) -> wgpu::BlendState {
        wgpu::BlendState {
            color: self.color.to_wgpu(),
            alpha: self.alpha.to_wgpu(),
        }
    }
}

impl BlendComponent {
    pub(crate) fn to_wgpu(&self) -> wgpu::BlendComponent {
        wgpu::BlendComponent {
            src_factor: self.src.to_wgpu(),
            dst_factor: self.dst.to_wgpu(),
            operation: self.op.to_wgpu(),
        }
    }
}

impl BlendFactor {
    pub(crate) fn to_wgpu(&self) -> wgpu::BlendFactor {
        match self {
            BlendFactor::Zero => wgpu::BlendFactor::Zero,
            BlendFactor::One => wgpu::BlendFactor::One,
            BlendFactor::SourceColor => wgpu::BlendFactor::Src,
            BlendFactor::InverseSourceColor => wgpu::BlendFactor::OneMinusSrc,
            BlendFactor::DestinationColor => wgpu::BlendFactor::Dst,
            BlendFactor::InverseDestinationColor => wgpu::BlendFactor::OneMinusDst,
            BlendFactor::SourceAlpha => wgpu::BlendFactor::SrcAlpha,
            BlendFactor::InverseSourceAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
            BlendFactor::DestinationAlpha => wgpu::BlendFactor::DstAlpha,
            BlendFactor::InverseDestinationAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
        }
    }
}

impl BlendOperation {
    pub(crate) fn to_wgpu(&self) -> wgpu::BlendOperation {
        match self {
            BlendOperation::Add => wgpu::BlendOperation::Add,
            BlendOperation::Subtract => wgpu::BlendOperation::Subtract,
            BlendOperation::ReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
            BlendOperation::Min => wgpu::BlendOperation::Min,
            BlendOperation::Max => wgpu::BlendOperation::Max,
        }
    }
}

pub fn wgpu_depth_stencil(
    depth: Option<DepthStencil>,
    stencil: Option<Stencil>,
) -> Option<wgpu::DepthStencilState> {
    if depth.is_none() && stencil.is_none() {
        return None;
    }

    let (depth_write_enabled, depth_compare) = match depth {
        None => (false, CompareFunction::Always),
        Some(depth) => (depth.write, depth.compare.to_wgpu()),
    };

    Some(wgpu::DepthStencilState {
        format: SURFACE_DEFAULT_DEPTH_FORMAT.to_wgpu(),
        depth_write_enabled,
        depth_compare,
        stencil: stencil.map_or(Default::default(), |stencil| {
            let stencil_face = wgpu::StencilFaceState {
                compare: stencil.compare.to_wgpu(),
                fail_op: stencil.stencil_fail.to_wgpu(),
                depth_fail_op: stencil.depth_fail.to_wgpu(),
                pass_op: stencil.pass.to_wgpu(),
            };

            wgpu::StencilState {
                front: stencil_face,
                back: stencil_face,
                read_mask: stencil.read_mask,
                write_mask: stencil.write_mask,
            }
        }),
        bias: Default::default(),
    })
}
