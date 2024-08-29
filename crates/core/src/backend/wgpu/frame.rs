use wgpu::{CommandEncoder, SurfaceTexture, TextureView};

pub struct DrawFrame {
    pub(crate) frame: SurfaceTexture,
    pub(crate) view: TextureView,
    pub(crate) encoder: CommandEncoder,
    pub(crate) dirty: bool,
    pub(crate) present_check: bool,
}
