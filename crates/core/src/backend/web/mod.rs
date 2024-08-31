#![cfg(target_arch = "wasm32")]

mod utils;
mod window;

#[cfg(all(feature = "clipboard", not(web_sys_unstable_apis)))]
compile_error!("feature \"clipboard\" requires web_sys_unstable_apis to be enabled\nsee https://rustwasm.github.io/wasm-bindgen/web-sys/unstable-apis.html");

use crate::app::WindowConfig;
use crate::backend::{BackendImpl, GfxBackendImpl};
use crate::builder::{AppBuilder, CleanupCb, InitCb, UpdateCb};
use crate::gfx::GfxBackend;
use crate::input::{KeyboardState, MouseState};
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use glam::Vec2;
use once_cell::sync::Lazy;
use window::WebWindow;

#[cfg(feature = "gamepad")]
use crate::input::GamepadState;

pub(crate) static BACKEND: Lazy<AtomicRefCell<WebBackend>> =
    Lazy::new(|| AtomicRefCell::new(WebBackend::default()));

pub fn run<S>(builder: AppBuilder<S>) -> Result<(), String>
where
    S: 'static,
{
    wasm_bindgen_futures::spawn_local(inner_run(builder));
    Ok(())
}

async fn inner_run<S>(builder: AppBuilder<S>)
where
    S: 'static,
{
    let AppBuilder {
        window: config,
        init_cb,
        update_cb,
        cleanup_cb,
    } = builder;
    log::debug!("Yes");

    let vsync = config.vsync;
    let size = config.size;

    let window = WebWindow::new(config).unwrap();
    let gfx = GfxBackend::init(&window, vsync, size).await.unwrap();
}

// - Backend
#[derive(Default)]
pub(crate) struct WebBackend {}

// hackish to allow the Lazy<T>, this is fine because wasm32 is not multithread
unsafe impl Sync for WebBackend {}
unsafe impl Send for WebBackend {}

impl BackendImpl<GfxBackend> for WebBackend {
    fn set_title(&mut self, title: &str) {
        todo!()
    }
    fn title(&self) -> String {
        todo!()
    }
    fn size(&self) -> Vec2 {
        todo!()
    }
    fn set_size(&mut self, size: Vec2) {
        todo!()
    }
    fn set_min_size(&mut self, size: Vec2) {
        todo!()
    }
    fn set_max_size(&mut self, size: Vec2) {
        todo!()
    }
    fn screen_size(&self) -> Vec2 {
        todo!()
    }
    fn is_fullscreen(&self) -> bool {
        todo!()
    }
    fn toggle_fullscreen(&mut self) {
        todo!()
    }
    fn dpi(&self) -> f32 {
        todo!()
    }
    fn position(&self) -> Vec2 {
        todo!()
    }
    fn set_position(&mut self, x: f32, y: f32) {
        todo!()
    }
    fn is_focused(&self) -> bool {
        todo!()
    }
    fn is_maximized(&self) -> bool {
        todo!()
    }
    fn is_minimized(&self) -> bool {
        todo!()
    }
    fn close(&mut self) {
        todo!()
    }

    // input
    fn mouse_state(&self) -> &MouseState {
        todo!()
    }
    fn keyboard_state(&self) -> &KeyboardState {
        todo!()
    }

    #[cfg(feature = "gamepad")]
    fn gamepad_state(&self) -> &GamepadState {
        todo!()
    }

    // gfx
    fn gfx(&mut self) -> &mut GfxBackend {
        todo!()
    }
}

pub(crate) fn get_backend() -> AtomicRef<'static, WebBackend> {
    BACKEND.borrow()
}
pub(crate) fn get_mut_backend() -> AtomicRefMut<'static, WebBackend> {
    BACKEND.borrow_mut()
}
