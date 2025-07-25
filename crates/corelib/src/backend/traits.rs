use crate::{
    gfx::{
        BindGroup, BindGroupDescriptor, Buffer, BufferDescriptor, GpuStats, Limits, RenderPipeline,
        RenderPipelineDescriptor, RenderTexture, RenderTextureDescriptor, Renderer, Sampler,
        SamplerDescriptor, Texture, TextureData, TextureDescriptor,
    },
    input::{KeyboardState, MouseState},
    math::{UVec2, Vec2},
};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

#[cfg(feature = "gamepad")]
use crate::input::GamepadState;

pub(crate) trait BackendImpl<G: GfxBackendImpl> {
    // Window
    fn set_title(&mut self, title: &str);
    fn title(&self) -> String;
    fn size(&self) -> Vec2;
    fn set_size(&mut self, size: Vec2);
    fn set_min_size(&mut self, size: Vec2);
    fn set_max_size(&mut self, size: Vec2);
    fn screen_size(&self) -> Vec2;
    fn is_fullscreen(&self) -> bool;
    fn toggle_fullscreen(&mut self);
    fn dpi(&self) -> f32;
    fn position(&self) -> Vec2;
    fn set_position(&mut self, x: f32, y: f32);
    fn is_focused(&self) -> bool;
    fn is_maximized(&self) -> bool;
    fn is_minimized(&self) -> bool;
    fn is_pixelated(&self) -> bool;
    fn close(&mut self);

    // input
    fn mouse_state(&self) -> &MouseState;
    fn set_cursor_lock(&mut self, lock: bool);
    fn is_cursor_locked(&self) -> bool;
    fn set_cursor_visible(&mut self, visible: bool);
    fn is_cursor_visible(&self) -> bool;

    fn keyboard_state(&self) -> &KeyboardState;

    #[cfg(feature = "gamepad")]
    fn gamepad_state(&self) -> &GamepadState;

    // gfx
    fn gfx(&mut self) -> &mut G;
}

pub(crate) trait GfxBackendImpl {
    async fn init<W>(
        window: &W,
        vsync: bool,
        win_size: UVec2,
        pixelated: bool,
    ) -> Result<Self, String>
    where
        Self: Sized,
        W: HasDisplayHandle + HasWindowHandle;

    #[cfg_attr(target_arch = "wasm32", allow(unused))]
    async fn update_surface<W>(
        &mut self,
        window: &W,
        vsync: bool,
        win_size: UVec2,
    ) -> Result<(), String>
    where
        Self: Sized,
        W: HasDisplayHandle + HasWindowHandle;

    fn prepare_frame(&mut self);
    fn present_frame(&mut self);

    fn render(&mut self, renderer: &Renderer) -> Result<(), String>;
    fn render_to(&mut self, texture: &RenderTexture, renderer: &Renderer) -> Result<(), String>;

    fn create_render_pipeline(
        &mut self,
        desc: RenderPipelineDescriptor,
    ) -> Result<RenderPipeline, String>;
    fn create_buffer(&mut self, desc: BufferDescriptor) -> Result<Buffer, String>;
    fn create_bind_group(&mut self, desc: BindGroupDescriptor) -> Result<BindGroup, String>;
    fn write_buffer(&mut self, buffer: &Buffer, offset: u64, data: &[u8]) -> Result<(), String>;
    fn create_sampler(&mut self, desc: SamplerDescriptor) -> Result<Sampler, String>;
    fn create_texture(
        &mut self,
        desc: TextureDescriptor,
        data: Option<TextureData>,
    ) -> Result<Texture, String>;
    fn write_texture(
        &mut self,
        texture: &Texture,
        offset: UVec2,
        size: UVec2,
        data: &[u8],
    ) -> Result<(), String>;
    fn create_render_texture(
        &mut self,
        desc: RenderTextureDescriptor,
    ) -> Result<RenderTexture, String>;
    fn limits(&self) -> Limits;
    fn stats(&self) -> GpuStats;
}
