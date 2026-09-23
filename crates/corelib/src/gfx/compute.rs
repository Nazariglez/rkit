use crate::gfx::{
    BindGroup, Buffer, BufferWriteMode, ComputePipeline, DispatchArgs, IndirectBuffer,
    validate_buffer_clear, validate_buffer_write,
};
use std::ops::Range;

#[derive(Default)]
pub struct Compute<'resource> {
    pub(crate) commands: Vec<ComputeCommand<'resource>>,
    pub(crate) error: Option<String>,
}

impl<'resource> Compute<'resource> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_buffer<'recorder>(
        &'recorder mut self,
        buffer: &'resource Buffer,
    ) -> ComputeBufferWrite<'recorder, 'resource> {
        ComputeBufferWrite {
            compute: self,
            buffer,
            offset: 0,
            bytes: None,
        }
    }

    pub fn clear_buffer(&mut self, buffer: &'resource Buffer) -> Result<&mut Self, String> {
        let size = u64::try_from(buffer.size())
            .map_err(|_| "Buffer size does not fit the supported range".to_string())?;
        self.clear_buffer_range(buffer, 0..size)
    }

    pub fn clear_buffer_range(
        &mut self,
        buffer: &'resource Buffer,
        bytes: Range<u64>,
    ) -> Result<&mut Self, String> {
        if let Err(error) = validate_buffer_clear(buffer, bytes.clone()) {
            return self.fail(error);
        }
        self.commands.push(ComputeCommand::Clear { buffer, bytes });
        Ok(self)
    }

    pub fn dispatch_for(
        &mut self,
        pipeline: &'resource ComputePipeline,
        workload: [u32; 3],
    ) -> &mut ComputeDispatch<'resource> {
        let workgroup_size = pipeline.workgroup_size();
        let workgroups = std::array::from_fn(|index| {
            let workload = workload[index];
            let size = workgroup_size[index];
            workload / size + u32::from(workload % size != 0)
        });
        self.dispatch_workgroups(pipeline, workgroups)
    }

    pub fn dispatch_workgroups(
        &mut self,
        pipeline: &'resource ComputePipeline,
        workgroups: [u32; 3],
    ) -> &mut ComputeDispatch<'resource> {
        self.dispatch(pipeline, ComputeWorkgroups::Direct(workgroups))
    }

    pub fn dispatch_indirect(
        &mut self,
        pipeline: &'resource ComputePipeline,
        arguments: &'resource IndirectBuffer<DispatchArgs>,
    ) -> &mut ComputeDispatch<'resource> {
        self.dispatch(pipeline, ComputeWorkgroups::Indirect(arguments))
    }

    fn dispatch(
        &mut self,
        pipeline: &'resource ComputePipeline,
        workgroups: ComputeWorkgroups<'resource>,
    ) -> &mut ComputeDispatch<'resource> {
        self.commands
            .push(ComputeCommand::Dispatch(ComputeDispatch {
                pipeline,
                workgroups,
                bind_groups: Vec::new(),
            }));
        let Some(ComputeCommand::Dispatch(dispatch)) = self.commands.last_mut() else {
            unreachable!();
        };
        dispatch
    }

    fn fail<T>(&mut self, error: String) -> Result<T, String> {
        self.error.get_or_insert(error.clone());
        Err(error)
    }
}

pub struct ComputeBufferWrite<'recorder, 'resource> {
    compute: &'recorder mut Compute<'resource>,
    buffer: &'resource Buffer,
    offset: u64,
    bytes: Option<Vec<u8>>,
}

impl<'recorder, 'resource> ComputeBufferWrite<'recorder, 'resource> {
    pub fn with_offset_bytes(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_data<D: bytemuck::Pod>(mut self, data: &[D]) -> Self {
        self.bytes = Some(bytemuck::cast_slice(data).to_vec());
        self
    }

    pub fn build(self) -> Result<(), String> {
        let Some(bytes) = self.bytes else {
            return self
                .compute
                .fail("Compute buffer writes require data".to_string());
        };
        if bytes.is_empty() {
            return self
                .compute
                .fail("Compute buffer writes require nonempty data".to_string());
        }
        if let Err(error) = validate_buffer_write(
            self.buffer,
            self.offset,
            bytes.len(),
            BufferWriteMode::Ordered,
        ) {
            return self.compute.fail(error);
        }
        self.compute.commands.push(ComputeCommand::Write {
            buffer: self.buffer,
            offset: self.offset,
            bytes,
        });
        Ok(())
    }
}

pub struct ComputeDispatch<'resource> {
    pub(crate) pipeline: &'resource ComputePipeline,
    pub(crate) workgroups: ComputeWorkgroups<'resource>,
    pub(crate) bind_groups: Vec<&'resource BindGroup>,
}

pub(crate) enum ComputeWorkgroups<'resource> {
    Direct([u32; 3]),
    Indirect(&'resource IndirectBuffer<DispatchArgs>),
}

impl<'resource> ComputeDispatch<'resource> {
    pub fn bindings(&mut self, groups: &[&'resource BindGroup]) -> &mut Self {
        self.bind_groups.clear();
        self.bind_groups.extend_from_slice(groups);
        self
    }
}

pub(crate) enum ComputeCommand<'resource> {
    Write {
        buffer: &'resource Buffer,
        offset: u64,
        bytes: Vec<u8>,
    },
    Clear {
        buffer: &'resource Buffer,
        bytes: Range<u64>,
    },
    Dispatch(ComputeDispatch<'resource>),
}
