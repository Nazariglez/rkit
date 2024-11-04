mod events;
mod input;
mod utils;
mod window;

use crate::backend::{BackendImpl, GfxBackendImpl};
use crate::builder::AppBuilder;
use crate::events::{CoreEvent, CORE_EVENTS_MAP};
use crate::gfx::GfxBackend;
use crate::input::{KeyboardState, MouseState};
use crate::math::Vec2;
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
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
        update_cb,
        resize_cb,
        cleanup_cb: _, // TODO cleanup
        ..
    } = builder;

    let vsync = config.vsync;
    let size = config.size;
    let pixelated = config.pixelated;

    let callback = Rc::new(RefCell::new(None));
    let win = WebWindow::new(config).unwrap();
    let close_requested = win.close_requested.clone();
    let gfx = GfxBackend::init(&win, vsync, size, pixelated)
        .await
        .unwrap();
    {
        let mut bck = get_mut_backend();
        bck.win = Some(win);
        bck.gfx = Some(gfx);
    }

    let mut runner = Runner {
        state: init_cb(),
        update: update_cb,
        resize: resize_cb,
    };

    CORE_EVENTS_MAP.borrow().trigger(CoreEvent::Init);

    let inner_callback = callback.clone();
    let win = web_sys::window().unwrap();
    *callback.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if *close_requested.borrow() {
            CORE_EVENTS_MAP.borrow().trigger(CoreEvent::CleanUp);
            return;
        }

        runner.tick();
        request_animation_frame(&win, inner_callback.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(
        &web_sys::window().unwrap(),
        callback.borrow().as_ref().unwrap(),
    );

    // TODO CoreEvent::Cleanup, and also stop request animation frame?
}

struct Runner<S> {
    state: S,
    update: Box<dyn FnMut(&mut S)>,
    resize: Box<dyn FnMut(&mut S)>,
}

impl<S> Runner<S> {
    fn tick(&mut self) {
        time::tick();

        // pre frame
        {
            CORE_EVENTS_MAP.borrow().trigger(CoreEvent::PreUpdate);
            self.process_events();
            let mut bck = get_mut_backend();
            bck.gfx().prepare_frame();
        }

        (*self.update)(&mut self.state);

        // post frame
        {
            let mut bck = get_mut_backend();
            bck.gfx().present_frame();
            CORE_EVENTS_MAP.borrow().trigger(CoreEvent::PostUpdate);

            bck.mouse_state.tick();
            bck.keyboard_state.tick();
        }
    }

    fn process_events(&mut self) {
        use events::Event::*;

        let mut events = get_mut_backend().take_events();
        #[allow(clippy::while_let_on_iterator)]
        while let Some(evt) = events.next() {
            match evt {
                MouseMove { pos, delta } => {
                    let mut bck = get_mut_backend();
                    bck.mouse_state.position = pos;
                    bck.mouse_state.moving = true;
                    bck.mouse_state.motion_delta = delta;
                    bck.mouse_state.cursor_on_screen = true;
                }
                MouseDown { btn } => {
                    let mut bck = get_mut_backend();
                    bck.mouse_state.press(btn);
                }
                MouseUp { btn } => {
                    let mut bck = get_mut_backend();
                    bck.mouse_state.release(btn);
                }
                MouseEnter => {
                    let mut bck = get_mut_backend();
                    bck.mouse_state.cursor_on_screen = true;
                }
                MouseLeave => {
                    let mut bck = get_mut_backend();
                    bck.mouse_state.cursor_on_screen = false;
                }
                MouseWheel { delta } => {
                    let mut bck = get_mut_backend();
                    bck.mouse_state.wheel_delta = delta;
                    bck.mouse_state.scrolling = true;
                }
                KeyUp { key } => {
                    let mut bck = get_mut_backend();
                    bck.keyboard_state.release(key);
                }
                KeyDown { key } => {
                    let mut bck = get_mut_backend();
                    bck.keyboard_state.press(key);
                }
                CharReceived { text } => {
                    let mut bck = get_mut_backend();
                    bck.keyboard_state.add_text(text.as_str());
                }
                WindowResize { size } => {
                    {
                        let mut bck = get_mut_backend();
                        bck.gfx.as_mut().unwrap().resize(size.x, size.y);
                    }
                    (*self.resize)(&mut self.state);
                }
            }
        }
    }
}

// - Backend
#[derive(Default)]
pub(crate) struct WebBackend {
    win: Option<WebWindow>,
    gfx: Option<GfxBackend>,

    pub(crate) mouse_state: MouseState,
    pub(crate) keyboard_state: KeyboardState,
}

// hackish to allow the Lazy<T>, this is fine because wasm32 is not multithread
unsafe impl Sync for WebBackend {}
unsafe impl Send for WebBackend {}

impl WebBackend {
    fn take_events(&mut self) -> events::EventIterator {
        self.win.as_mut().unwrap().events.take()
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
        self.win
            .as_mut()
            .unwrap()
            .set_min_size(size.x as _, size.y as _);
    }
    fn set_max_size(&mut self, size: Vec2) {
        self.win
            .as_mut()
            .unwrap()
            .set_max_size(size.x as _, size.y as _);
    }
    fn screen_size(&self) -> Vec2 {
        todo!()
    }
    fn is_fullscreen(&self) -> bool {
        self.win.as_ref().unwrap().is_fullscreen()
    }
    fn toggle_fullscreen(&mut self) {
        self.win.as_mut().unwrap().toggle_fullscreen();
    }
    fn dpi(&self) -> f32 {
        self.win.as_ref().unwrap().dpi
    }
    fn position(&self) -> Vec2 {
        todo!()
    }
    fn set_position(&mut self, _x: f32, _y: f32) {
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
    fn is_pixelated(&self) -> bool {
        self.win.as_ref().unwrap().pixelated
    }
    fn close(&mut self) {
        self.win.as_ref().unwrap().exit();
    }

    // input
    fn mouse_state(&self) -> &MouseState {
        &self.mouse_state
    }

    fn set_cursor_lock(&mut self, lock: bool) {
        if self.is_cursor_locked() == lock {
            return;
        }

        self.win
            .as_mut()
            .unwrap()
            .cursor_lock_request
            .replace(Some(lock));
    }

    fn is_cursor_locked(&self) -> bool {
        self.win
            .as_ref()
            .map_or(false, |w| *w.cursor_locked.borrow())
    }
    fn set_cursor_visible(&mut self, visible: bool) {
        // TODO store last mode to put it back instead of default
        let mode = if visible { "default" } else { "none" };
        self.win
            .as_mut()
            .unwrap()
            .canvas
            .style()
            .set_property("cursor", mode)
            .unwrap();
    }
    fn is_cursor_visible(&self) -> bool {
        let current = self
            .win
            .as_ref()
            .unwrap()
            .canvas
            .style()
            .get_property_value("cursor")
            .unwrap_or_default();
        current != "none"
    }
    fn keyboard_state(&self) -> &KeyboardState {
        &self.keyboard_state
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
