#![cfg(target_arch = "wasm32")]

mod events;
mod input;
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
use glam::{vec2, Vec2};
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::rc::Rc;
use utils::request_animation_frame;
use wasm_bindgen::closure::Closure;
use window::WebWindow;

#[cfg(feature = "gamepad")]
use crate::input::GamepadState;
use crate::time;

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
        mut update_cb,
        cleanup_cb,
    } = builder;
    log::debug!("Yes");

    let vsync = config.vsync;
    let size = config.size;

    let callback = Rc::new(RefCell::new(None));
    let win = WebWindow::new(config, callback.clone()).unwrap();
    let gfx = GfxBackend::init(&win, vsync, size).await.unwrap();
    {
        let mut bck = get_mut_backend();
        bck.win = Some(win);
        bck.gfx = Some(gfx);
    }

    let mut runner = Runner {
        state: init_cb(),
        update: update_cb,
    };

    let inner_callback = callback.clone();
    let win = web_sys::window().unwrap();
    *callback.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        runner.tick();
        request_animation_frame(&win, inner_callback.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(
        &web_sys::window().unwrap(),
        callback.borrow().as_ref().unwrap(),
    );
}

struct Runner<S> {
    state: S,
    update: Box<dyn FnMut(&mut S)>,
}

impl<S> Runner<S> {
    fn tick(&mut self) {
        time::tick();

        // process events
        get_mut_backend().process_events();

        get_mut_backend().gfx().prepare_frame();
        (*self.update)(&mut self.state);
        get_mut_backend().gfx().present_frame();

        get_mut_backend().mouse_state.tick();
    }
}

// - Backend
#[derive(Default)]
pub(crate) struct WebBackend {
    win: Option<WebWindow>,
    gfx: Option<GfxBackend>,

    pub(crate) mouse_state: MouseState,
}

// hackish to allow the Lazy<T>, this is fine because wasm32 is not multithread
unsafe impl Sync for WebBackend {}
unsafe impl Send for WebBackend {}

impl WebBackend {
    fn process_events(&mut self) {
        use events::Event::*;

        let mut events = self.win.as_mut().unwrap().events.take();
        while let Some(evt) = events.next() {
            // log::warn!("{:?}", evt);
            match evt {
                MouseMove { pos } => {
                    self.mouse_state.position = pos;
                    self.mouse_state.moving = true;
                    // TODO motion_delta
                    // self.mouse_state.motion_delta = N;
                }
                MouseDown { btn, pos } => {
                    self.mouse_state.position = pos;
                    self.mouse_state.press(btn);
                }
                MouseUp { btn, pos } => {
                    self.mouse_state.position = pos;
                    self.mouse_state.release(btn);
                }
                _ => {}
            }
        }
    }
}

impl BackendImpl<GfxBackend> for WebBackend {
    fn set_title(&mut self, title: &str) {
        self.win.as_mut().unwrap().set_title(title);
    }
    fn title(&self) -> String {
        self.win.as_ref().unwrap().title().to_owned()
    }
    fn size(&self) -> Vec2 {
        self.win.as_ref().map_or(Vec2::ZERO, |w| w.size())
    }
    fn set_size(&mut self, size: Vec2) {
        self.win
            .as_mut()
            .unwrap()
            .set_size(size.x as _, size.y as _);
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
        &self.mouse_state
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
        self.gfx.as_mut().unwrap()
    }
}

pub(crate) fn get_backend() -> AtomicRef<'static, WebBackend> {
    BACKEND.borrow()
}
pub(crate) fn get_mut_backend() -> AtomicRefMut<'static, WebBackend> {
    BACKEND.borrow_mut()
}
