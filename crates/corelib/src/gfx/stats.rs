// TODO: impl all here (only draw calls is added)
#[derive(Copy, Clone, Default)]
pub struct GpuStats {
    /// Number of draw calls
    pub draw_calls: usize,
    /// Number of read_pixels callas
    pub read_pixels: usize,
    /// Number of textures updated
    pub texture_updates: usize,
    /// Number of textures created
    pub texture_creation: usize,
    /// Number of buffers updated
    pub buffer_updates: usize,
    /// Number of buffers created
    pub buffer_creation: usize,
    /// Any other interaction with the GPU
    pub misc: usize,
}