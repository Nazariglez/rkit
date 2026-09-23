use crate::{
    backend::{BackendImpl, GfxBackendImpl, get_mut_backend},
    gfx::{Buffer, Texture, TextureFormat},
};
use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub enum Readback {
    Buffer(Vec<u8>),
    Texture(TextureReadback),
}

#[derive(Debug)]
pub struct TextureReadback {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub bytes_per_row: u32,
}

pub(crate) struct TicketState {
    pub(crate) ready: Option<Result<Readback, String>>,
    pub(crate) taken: bool,
    pub(crate) cancelled: bool,
    pub(crate) mapped: Option<Result<(), String>>,
}

impl TicketState {
    pub(crate) fn pending() -> Self {
        Self {
            ready: None,
            taken: false,
            cancelled: false,
            mapped: None,
        }
    }
}

pub struct ReadbackTicket {
    pub(crate) id: u64,
    pub(crate) state: Arc<Mutex<TicketState>>,
}

impl std::fmt::Debug for ReadbackTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadbackTicket")
            .field("id", &self.id)
            .finish()
    }
}

impl ReadbackTicket {
    pub(crate) fn new(id: u64, state: Arc<Mutex<TicketState>>) -> Self {
        Self { id, state }
    }

    pub fn try_take(&mut self) -> Result<Option<Readback>, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Readback ticket state is unavailable".to_string())?;
        let Some(readback) = state.ready.take() else {
            return Ok(None);
        };
        state.taken = true;
        readback.map(Some)
    }
}

impl Drop for ReadbackTicket {
    fn drop(&mut self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if !state.taken && state.ready.is_none() {
            state.cancelled = true;
        }
    }
}

pub struct BufferReadbackBuilder<'a> {
    buffer: &'a Buffer,
    bytes: Range<u64>,
}

impl<'a> BufferReadbackBuilder<'a> {
    pub(crate) fn new(buffer: &'a Buffer) -> Self {
        Self {
            buffer,
            bytes: 0..buffer.size() as u64,
        }
    }

    pub fn with_byte_range(mut self, bytes: Range<u64>) -> Self {
        self.bytes = bytes;
        self
    }

    pub fn build(self) -> Result<ReadbackTicket, String> {
        get_mut_backend().gfx().read_buffer(self.buffer, self.bytes)
    }
}

pub struct TextureReadbackBuilder<'a> {
    texture: &'a Texture,
}

impl<'a> TextureReadbackBuilder<'a> {
    pub(crate) fn new(texture: &'a Texture) -> Self {
        Self { texture }
    }

    pub fn build(self) -> Result<ReadbackTicket, String> {
        get_mut_backend().gfx().read_texture(self.texture)
    }
}
