use crate::gfx::{
    BufferId, BufferUsage, CompareMode, CullMode, IndexFormat, Primitive, StencilAction,
    VertexFormat, VertexStepMode,
};
use atomic_refcell::AtomicRefCell;
use std::sync::Arc;
use wgpu::{Buffer as RawBuffer, BufferUsages, StencilOperation};

pub struct InnerBuffer {
    pub size: usize,
    pub raw: Arc<RawBuffer>,
}

#[derive(Clone)]
pub struct Buffer {
    pub(crate) id: BufferId,
    // NOTE: this ugly double Arc is because to create the raw binding we need a reference
    // to Buffer that cannot be under a borrow because lifetime issues, and the atomic
    // refcell is necessary to update the buffer when the size is too small on write
    // operations if the performance is not acceptable we can think about unsafe I guess
    pub(crate) raw: Arc<AtomicRefCell<Arc<RawBuffer>>>,
    pub(crate) usage: BufferUsage,
    pub(crate) write: bool,
    pub(crate) size: usize,
    pub(crate) inner_label: Arc<String>,
}

impl Buffer {
    pub fn id(&self) -> BufferId {
        self.id
    }

    pub fn usage(&self) -> BufferUsage {
        self.usage
    }

    pub fn is_writable(&self) -> bool {
        self.write
    }

    pub fn len(&self) -> usize {
        self.size
    }
}

impl BufferUsage {
    pub(crate) fn to_wgpu(&self) -> wgpu::BufferUsages {
        match self {
            BufferUsage::Vertex => BufferUsages::VERTEX,
            BufferUsage::Index => BufferUsages::INDEX,
            BufferUsage::Uniform => BufferUsages::UNIFORM,
        }
    }
}

impl VertexFormat {
    pub(crate) fn to_wgpu(&self) -> wgpu::VertexFormat {
        match self {
            VertexFormat::UInt8x2 => wgpu::VertexFormat::Uint8x2,
            VertexFormat::UInt8x4 => wgpu::VertexFormat::Uint8x4,
            VertexFormat::Int8x2 => wgpu::VertexFormat::Sint8x2,
            VertexFormat::Int8x4 => wgpu::VertexFormat::Sint8x4,
            VertexFormat::U8x2norm => wgpu::VertexFormat::Unorm8x2,
            VertexFormat::U8x4norm => wgpu::VertexFormat::Unorm8x4,
            VertexFormat::I8x2norm => wgpu::VertexFormat::Snorm8x2,
            VertexFormat::I8x4norm => wgpu::VertexFormat::Snorm8x4,
            VertexFormat::UInt16x2 => wgpu::VertexFormat::Uint16x2,
            VertexFormat::UInt16x4 => wgpu::VertexFormat::Uint16x4,
            VertexFormat::Int16x2 => wgpu::VertexFormat::Sint16x2,
            VertexFormat::Int16x4 => wgpu::VertexFormat::Sint16x4,
            VertexFormat::U16x2norm => wgpu::VertexFormat::Unorm16x2,
            VertexFormat::U16x4norm => wgpu::VertexFormat::Unorm16x4,
            VertexFormat::Int16x2norm => wgpu::VertexFormat::Snorm16x2,
            VertexFormat::Int16x4norm => wgpu::VertexFormat::Snorm16x4,
            VertexFormat::Float16x2 => wgpu::VertexFormat::Float16x2,
            VertexFormat::Float16x4 => wgpu::VertexFormat::Float16x4,
            VertexFormat::Float32 => wgpu::VertexFormat::Float32,
            VertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
            VertexFormat::Float32x3 => wgpu::VertexFormat::Float32x3,
            VertexFormat::Float32x4 => wgpu::VertexFormat::Float32x4,
            VertexFormat::UInt32 => wgpu::VertexFormat::Uint32,
            VertexFormat::UInt32x2 => wgpu::VertexFormat::Uint32x2,
            VertexFormat::UInt32x3 => wgpu::VertexFormat::Uint32x3,
            VertexFormat::UInt32x4 => wgpu::VertexFormat::Uint32x4,
            VertexFormat::Int32 => wgpu::VertexFormat::Sint32,
            VertexFormat::Int32x2 => wgpu::VertexFormat::Sint32x2,
            VertexFormat::Int32x3 => wgpu::VertexFormat::Sint32x3,
            VertexFormat::Int32x4 => wgpu::VertexFormat::Sint32x4,
        }
    }
}

impl VertexStepMode {
    pub fn to_wgpu(self) -> wgpu::VertexStepMode {
        match self {
            VertexStepMode::Vertex => wgpu::VertexStepMode::Vertex,
            VertexStepMode::Instance => wgpu::VertexStepMode::Instance,
        }
    }
}

impl IndexFormat {
    pub(crate) fn to_wgpu(&self) -> wgpu::IndexFormat {
        match self {
            IndexFormat::UInt16 => wgpu::IndexFormat::Uint16,
            IndexFormat::UInt32 => wgpu::IndexFormat::Uint32,
        }
    }
}

impl Primitive {
    pub(crate) fn to_wgpu(&self) -> wgpu::PrimitiveTopology {
        match self {
            Primitive::Points => wgpu::PrimitiveTopology::PointList,
            Primitive::Lines => wgpu::PrimitiveTopology::LineList,
            Primitive::LineStrip => wgpu::PrimitiveTopology::LineStrip,
            Primitive::Triangles => wgpu::PrimitiveTopology::TriangleList,
            Primitive::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
        }
    }
}

impl CullMode {
    pub(crate) fn to_wgpu(&self) -> wgpu::Face {
        match self {
            CullMode::Front => wgpu::Face::Front,
            CullMode::Back => wgpu::Face::Back,
        }
    }
}

impl CompareMode {
    pub(crate) fn to_wgpu(&self) -> wgpu::CompareFunction {
        match self {
            CompareMode::Never => wgpu::CompareFunction::Never,
            CompareMode::Less => wgpu::CompareFunction::Less,
            CompareMode::Equal => wgpu::CompareFunction::Equal,
            CompareMode::LEqual => wgpu::CompareFunction::LessEqual,
            CompareMode::Greater => wgpu::CompareFunction::Greater,
            CompareMode::NotEqual => wgpu::CompareFunction::NotEqual,
            CompareMode::GEqual => wgpu::CompareFunction::GreaterEqual,
            CompareMode::Always => wgpu::CompareFunction::Always,
        }
    }
}

impl StencilAction {
    pub(crate) fn to_wgpu(&self) -> wgpu::StencilOperation {
        match self {
            StencilAction::Keep => wgpu::StencilOperation::Keep,
            StencilAction::Zero => wgpu::StencilOperation::Zero,
            StencilAction::Replace => wgpu::StencilOperation::Replace,
            StencilAction::Increment => wgpu::StencilOperation::IncrementClamp,
            StencilAction::IncrementWrap => wgpu::StencilOperation::IncrementWrap,
            StencilAction::Decrement => wgpu::StencilOperation::DecrementClamp,
            StencilAction::DecrementWrap => wgpu::StencilOperation::DecrementWrap,
            StencilAction::Invert => wgpu::StencilOperation::Invert,
        }
    }
}
