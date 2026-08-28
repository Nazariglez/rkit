use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use glam::Vec2;
use once_cell::sync::Lazy;
use spin_sleep_util::Interval;

use crate::{
    backend::traits::{BackendImpl, GfxBackendImpl},
    builder::AppBuilder,
    events::{CORE_EVENTS_MAP, CoreEvent},
    input::{KeyboardState, MouseState},
};

pub(crate) static BACKEND: Lazy<AtomicRefCell<HeadlessBackend>> =
    Lazy::new(|| AtomicRefCell::new(HeadlessBackend::default()));

#[derive(Debug, Default)]
pub(crate) struct HeadlessGfx;

impl GfxBackendImpl for HeadlessGfx {
    fn frame_size(&self) -> glam::UVec2 {
        glam::UVec2::ZERO
    }

    fn prepare_frame(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn present_frame(&mut self) {}

    fn render(&mut self, _renderer: &crate::gfx::Renderer) -> Result<(), String> {
        Ok(())
    }

    fn render_to(
        &mut self,
        _texture: &super::gfx::RenderTexture,
        _renderer: &crate::gfx::Renderer,
    ) -> Result<(), String> {
        Ok(())
    }

    fn create_render_pipeline(
        &mut self,
        _desc: crate::gfx::RenderPipelineDescriptor,
    ) -> Result<super::gfx::RenderPipeline, String> {
        unreachable!()
    }

    fn create_stencil_variant(
        &mut self,
        _base: &crate::gfx::RenderPipeline,
        _stencil: crate::gfx::Stencil,
    ) -> Result<crate::gfx::RenderPipeline, String> {
        unreachable!()
    }

    fn create_buffer(
        &mut self,
        _desc: crate::gfx::BufferDescriptor,
    ) -> Result<super::gfx::Buffer, String> {
        unreachable!()
    }

    fn create_bind_group(
        &mut self,
        _desc: crate::gfx::BindGroupDescriptor,
    ) -> Result<super::gfx::BindGroup, String> {
        unreachable!()
    }

    fn write_buffer(
        &mut self,
        _buffer: &super::gfx::Buffer,
        _offset: u64,
        _data: &[u8],
    ) -> Result<(), String> {
        unreachable!()
    }

    fn create_sampler(
        &mut self,
        _desc: crate::gfx::SamplerDescriptor,
    ) -> Result<super::gfx::Sampler, String> {
        unreachable!()
    }

    fn create_texture(
        &mut self,
        _desc: crate::gfx::TextureDescriptor,
        _data: Option<crate::gfx::TextureData>,
    ) -> Result<super::gfx::Texture, String> {
        unreachable!()
    }

    fn write_texture(
        &mut self,
        _texture: &super::gfx::Texture,
        _offset: glam::UVec2,
        _size: glam::UVec2,
        _data: &[u8],
    ) -> Result<(), String> {
        unreachable!()
    }

    fn create_render_texture(
        &mut self,
        _desc: crate::gfx::RenderTextureDescriptor,
    ) -> Result<super::gfx::RenderTexture, String> {
        unreachable!()
    }

    fn limits(&self) -> crate::gfx::Limits {
        unreachable!()
    }

    fn stats(&self) -> crate::gfx::GpuStats {
        unreachable!()
    }
}

#[derive(Default)]
pub(crate) struct HeadlessBackend {
    request_close: bool,
    mouse_state: MouseState,
    keyboard_state: KeyboardState,
    gfx: HeadlessGfx,
}

impl BackendImpl<HeadlessGfx> for HeadlessBackend {
    fn set_title(&mut self, title: &str) {
        unreachable!()
    }

    fn title(&self) -> String {
        unreachable!()
    }

    fn size(&self) -> glam::Vec2 {
        Vec2::ZERO
    }

    fn set_size(&mut self, size: glam::Vec2) {
        unreachable!()
    }

    fn set_min_size(&mut self, size: glam::Vec2) {
        unreachable!()
    }

    fn set_max_size(&mut self, size: glam::Vec2) {
        unreachable!()
    }

    fn screen_size(&self) -> glam::Vec2 {
        unreachable!()
    }

    fn is_fullscreen(&self) -> bool {
        unreachable!()
    }

    fn toggle_fullscreen(&mut self) {
        unreachable!()
    }

    fn dpi(&self) -> f32 {
        unreachable!()
    }

    fn position(&self) -> glam::Vec2 {
        unreachable!()
    }

    fn set_position(&mut self, x: f32, y: f32) {
        unreachable!()
    }

    fn is_focused(&self) -> bool {
        unreachable!()
    }

    fn is_maximized(&self) -> bool {
        unreachable!()
    }

    fn is_minimized(&self) -> bool {
        unreachable!()
    }

    fn is_pixelated(&self) -> bool {
        unreachable!()
    }

    fn close(&mut self) {
        self.request_close = true;
    }

    fn mouse_state(&self) -> &crate::input::MouseState {
        &self.mouse_state
    }

    fn set_cursor_lock(&mut self, lock: bool) {
        unreachable!()
    }

    fn is_cursor_locked(&self) -> bool {
        false
    }

    fn set_cursor_visible(&mut self, visible: bool) {
        unreachable!()
    }

    fn is_cursor_visible(&self) -> bool {
        false
    }

    fn keyboard_state(&self) -> &crate::input::KeyboardState {
        &self.keyboard_state
    }

    fn gfx(&mut self) -> &mut HeadlessGfx {
        &mut self.gfx
    }
}

struct Runner<S> {
    state: S,
    update: Box<dyn FnMut(&mut S)>,
    interval: Interval,
}

impl<S> Runner<S> {
    fn tick(&mut self) -> Result<bool, String> {
        crate::time::tick();

        // pre frame
        {
            CORE_EVENTS_MAP.borrow().trigger(CoreEvent::PreUpdate);
            self.process_events();
            let mut bck = get_mut_backend();
            bck.gfx().prepare_frame()?;
        }

        (*self.update)(&mut self.state);

        // post frame
        {
            let mut bck = get_mut_backend();
            bck.gfx().present_frame();
            CORE_EVENTS_MAP.borrow().trigger(CoreEvent::PostUpdate);
        }

        self.interval.tick();

        Ok(get_backend().request_close)
    }

    fn process_events(&mut self) {
        // do nothing?
    }
}

pub(crate) fn get_backend() -> AtomicRef<'static, HeadlessBackend> {
    BACKEND.borrow()
}
pub(crate) fn get_mut_backend() -> AtomicRefMut<'static, HeadlessBackend> {
    BACKEND.borrow_mut()
}

pub fn run<S>(builder: AppBuilder<S>) -> Result<(), String>
where
    S: 'static,
{
    let AppBuilder {
        window,
        cleanup_cb,
        init_cb,
        update_cb,
        ..
    } = builder;

    let run_fps = window.max_fps.unwrap_or(60);
    let interval = spin_sleep_util::interval(std::time::Duration::from_secs(1) / run_fps as u32);

    let mut runner = Runner {
        state: init_cb(),
        update: update_cb,
        interval,
    };

    let run_result = loop {
        match runner.tick() {
            Ok(true) => break Ok(()),
            Ok(false) => {}
            Err(err) => break Err(err),
        }
    };

    // at this point the runner is not in use, the app is closing
    cleanup_cb(&mut runner.state);

    CORE_EVENTS_MAP.borrow().trigger(CoreEvent::CleanUp);

    run_result
}
