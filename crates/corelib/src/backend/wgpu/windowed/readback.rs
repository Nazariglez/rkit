use crate::gfx::{Readback, ReadbackTicket, TextureFormat, TextureReadback, TicketState};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Device, Extent3d, MapMode,
    Origin3d, Queue, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo,
};

const MAX_IN_FLIGHT_READBACKS: usize = 8;

pub(crate) struct ReadbackManager {
    next_id: u64,
    slots: HashMap<u64, ReadbackSlot>,
    terminal_error: Option<String>,
}

struct ReadbackSlot {
    staging: Buffer,
    state: Arc<Mutex<TicketState>>,
    layout: ReadbackLayout,
}

enum ReadbackLayout {
    Buffer,
    Texture {
        width: u32,
        height: u32,
        format: TextureFormat,
        tight_row: u32,
        padded_row: u32,
    },
}

impl Default for ReadbackManager {
    fn default() -> Self {
        Self {
            next_id: 0,
            slots: HashMap::new(),
            terminal_error: None,
        }
    }
}

impl ReadbackManager {
    pub(crate) fn read_buffer(
        &mut self,
        device: &Device,
        queue: &Queue,
        source: &Buffer,
        offset: u64,
        bytes: u64,
    ) -> Result<ReadbackTicket, String> {
        let staging = self.staging_buffer(device, bytes)?;
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Buffer Readback"),
        });
        encoder.copy_buffer_to_buffer(source, offset, &staging, 0, bytes);
        queue.submit(Some(encoder.finish()));
        Ok(self.start_mapping(staging, ReadbackLayout::Buffer))
    }

    pub(crate) fn read_texture(
        &mut self,
        device: &Device,
        queue: &Queue,
        source: &wgpu::Texture,
        width: u32,
        height: u32,
        format: TextureFormat,
        tight_row: u32,
    ) -> Result<ReadbackTicket, String> {
        let padded_row = align_row(tight_row)?;
        let bytes = u64::from(padded_row)
            .checked_mul(u64::from(height))
            .ok_or_else(|| "Texture readback allocation size overflows".to_string())?;
        let staging = self.staging_buffer(device, bytes)?;
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Texture Readback"),
        });
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: source,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &staging,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        Ok(self.start_mapping(
            staging,
            ReadbackLayout::Texture {
                width,
                height,
                format,
                tight_row,
                padded_row,
            },
        ))
    }

    pub(crate) fn progress(&mut self, device: &Device, device_loss: Option<String>) -> bool {
        let needs_redraw = !self.slots.is_empty();
        if let Some(error) = device_loss {
            self.stop(error);
            return needs_redraw;
        }
        if let Err(error) = device.poll(wgpu::PollType::Poll) {
            log::warn!("Readback device polling did not progress: {error}");
            return needs_redraw;
        }

        let complete = self
            .slots
            .iter()
            .filter_map(|(&id, slot)| {
                slot.state
                    .lock()
                    .ok()
                    .and_then(|state| state.mapped.as_ref().map(|_| id))
            })
            .collect::<Vec<_>>();
        for id in complete {
            self.finish(id);
        }
        needs_redraw
    }

    pub(crate) fn device_lost(&mut self, error: String) {
        self.stop(error);
    }

    pub(crate) fn shutdown(&mut self) {
        self.stop("Readback manager shut down".to_string());
    }

    fn staging_buffer(&self, device: &Device, size: u64) -> Result<Buffer, String> {
        if let Some(error) = &self.terminal_error {
            return Err(error.clone());
        }
        if self.slots.len() >= MAX_IN_FLIGHT_READBACKS {
            return Err("Too many readback requests are in flight".to_string());
        }
        if size == 0 {
            return Err("Readback size must be nonzero".to_string());
        }
        let max_buffer_size = device.limits().max_buffer_size;
        if size > max_buffer_size {
            return Err(format!(
                "Readback staging allocation of {size} bytes exceeds the device maximum of {max_buffer_size} bytes"
            ));
        }
        Ok(device.create_buffer(&BufferDescriptor {
            label: Some("Readback Staging"),
            size,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }))
    }

    fn start_mapping(&mut self, staging: Buffer, layout: ReadbackLayout) -> ReadbackTicket {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let state = Arc::new(Mutex::new(TicketState::pending()));
        let callback_state = state.clone();
        staging.slice(..).map_async(MapMode::Read, move |mapped| {
            if let Ok(mut state) = callback_state.lock() {
                state.mapped =
                    Some(mapped.map_err(|error| format!("Readback mapping failed: {error}")));
            }
        });
        self.slots.insert(
            id,
            ReadbackSlot {
                staging,
                state: state.clone(),
                layout,
            },
        );
        ReadbackTicket::new(id, state)
    }

    fn finish(&mut self, id: u64) {
        let readback = {
            let Some(slot) = self.slots.get(&id) else {
                return;
            };
            let state = slot.state.lock().ok();
            let Some(state) = state else {
                return;
            };
            if state.cancelled {
                None
            } else {
                let Some(mapped) = state.mapped.clone() else {
                    return;
                };
                drop(state);
                Some(mapped.and_then(|()| read_slot(slot)))
            }
        };
        self.terminal(id, readback);
    }

    fn stop(&mut self, error: String) {
        if self.terminal_error.is_none() {
            self.terminal_error = Some(error.clone());
        }
        let ids = self.slots.keys().copied().collect::<Vec<_>>();
        for id in ids {
            self.terminal(id, Some(Err(error.clone())));
        }
    }

    fn terminal(&mut self, id: u64, readback: Option<Result<Readback, String>>) {
        let Some(slot) = self.slots.remove(&id) else {
            return;
        };
        let mapped_successfully = slot
            .state
            .lock()
            .ok()
            .and_then(|state| state.mapped.as_ref().map(Result::is_ok))
            .unwrap_or(false);
        if mapped_successfully {
            slot.staging.unmap();
        }
        if let Ok(mut state) = slot.state.lock()
            && !state.cancelled
            && let Some(readback) = readback
        {
            state.ready = Some(readback);
        }
    }
}

impl Drop for ReadbackManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn align_row(tight_row: u32) -> Result<u32, String> {
    let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    tight_row
        .checked_add(alignment - 1)
        .map(|row| row / alignment * alignment)
        .ok_or_else(|| "Texture readback row size overflows".to_string())
}

fn read_slot(slot: &ReadbackSlot) -> Result<Readback, String> {
    let mapped = slot
        .staging
        .slice(..)
        .get_mapped_range()
        .map_err(|error| format!("Readback mapped range is unavailable: {error}"))?;
    match slot.layout {
        ReadbackLayout::Buffer => Ok(Readback::Buffer(mapped.to_vec())),
        ReadbackLayout::Texture {
            width,
            height,
            format,
            tight_row,
            padded_row,
        } => {
            let length = usize::try_from(tight_row)
                .ok()
                .and_then(|row| row.checked_mul(height as usize))
                .ok_or_else(|| "Texture readback result size overflows".to_string())?;
            let mut bytes = Vec::with_capacity(length);
            for row in mapped
                .chunks_exact(padded_row as usize)
                .take(height as usize)
            {
                bytes.extend_from_slice(&row[..tight_row as usize]);
            }
            Ok(Readback::Texture(TextureReadback {
                bytes,
                width,
                height,
                format,
                bytes_per_row: tight_row,
            }))
        }
    }
}
