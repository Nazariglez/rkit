use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use once_cell::sync::Lazy;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window, WindowId};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys;

use super::backend::BackendImpl;
use crate::builder::UpdateCb;
use math::{vec2, Vec2};

// TODO, screen_size, positions etc... must be logical or physical pixels?

pub(crate) static BACKEND: Lazy<AtomicRefCell<WinitBackend>> =
    Lazy::new(|| AtomicRefCell::new(WinitBackend::default()));

#[derive(Default)]
pub(crate) struct WinitBackend {
    window: Option<Window>,
}

impl BackendImpl for WinitBackend {
    #[inline]
    fn set_title(&mut self, title: &str) {
        debug_assert!(self.window.is_some(), "Window must be present");
        self.window.as_mut().unwrap().set_title(title);
    }

    #[inline]
    fn title(&self) -> String {
        debug_assert!(self.window.is_some(), "Window must be present");
        self.window.as_ref().unwrap().title()
    }

    #[inline]
    fn size(&self) -> Vec2 {
        debug_assert!(self.window.is_some(), "Window must be present");
        let w = self.window.as_ref().unwrap();
        let size = w.inner_size().to_logical::<f32>(w.scale_factor());
        vec2(size.width, size.height)
    }

    #[inline]
    fn set_size(&mut self, size: Vec2) {
        debug_assert!(self.window.is_some(), "Window must be present");
        let _ = self
            .window
            .as_mut()
            .unwrap()
            .request_inner_size(LogicalSize::new(size.x, size.y));
    }

    #[inline]
    fn is_fullscreen(&self) -> bool {
        debug_assert!(self.window.is_some(), "Window must be present");
        self.window.as_ref().unwrap().fullscreen().is_some()
    }

    #[inline]
    fn toggle_fullscreen(&mut self) {
        debug_assert!(self.window.is_some(), "Window must be present");
        let is_not_fullscreen = !self.is_fullscreen();
        let mode = is_not_fullscreen.then_some(Fullscreen::Borderless(None));
        self.window.as_mut().unwrap().set_fullscreen(mode);
    }

    #[inline]
    fn screen_size(&self) -> Vec2 {
        debug_assert!(self.window.is_some(), "Window must be present");
        let w = self.window.as_ref().unwrap();
        let scale = w.scale_factor();
        w.current_monitor().map_or(Vec2::ZERO, |m| {
            let m = m.size().to_logical::<f32>(scale);
            vec2(m.width, m.height)
        })
    }

    #[inline]
    fn dpi(&self) -> f32 {
        debug_assert!(self.window.is_some(), "Window must be present");
        self.window.as_ref().unwrap().scale_factor() as _
    }

    #[inline]
    fn position(&self) -> Vec2 {
        debug_assert!(self.window.is_some(), "Window must be present");
        self.window
            .as_ref()
            .unwrap()
            .outer_position()
            .map_or(Vec2::ZERO, |p| vec2(p.x as _, p.y as _))
    }

    #[inline]
    fn set_position(&mut self, x: f32, y: f32) {
        debug_assert!(self.window.is_some(), "Window must be present");
        self.window
            .as_mut()
            .unwrap()
            .set_outer_position(PhysicalPosition::new(x, y))
    }

    #[inline]
    fn is_focused(&self) -> bool {
        debug_assert!(self.window.is_some(), "Window must be present");
        self.window.as_ref().unwrap().has_focus()
    }

    #[inline]
    fn is_maximized(&self) -> bool {
        debug_assert!(self.window.is_some(), "Window must be present");
        self.window.as_ref().unwrap().is_maximized()
    }

    #[inline]
    fn is_minimized(&self) -> bool {
        debug_assert!(self.window.is_some(), "Window must be present");
        self.window
            .as_ref()
            .unwrap()
            .is_minimized()
            .unwrap_or_default()
    }
}

struct Runner<S> {
    state: S,
    update: Box<dyn FnMut(&mut S)>,
}

impl<S> ApplicationHandler for Runner<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut attrs =
            Window::default_attributes().with_inner_size(LogicalSize::new(800.0, 600.0));

        #[cfg(target_arch = "wasm32")]
        {
            attrs = attrs.with_append(true);
        }
        BACKEND.borrow_mut().window = Some(event_loop.create_window(attrs).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                // Draw.
                (*self.update)(&mut self.state);

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                BACKEND
                    .borrow_mut()
                    .window
                    .as_ref()
                    .unwrap()
                    .request_redraw();
            }
            _ => (),
        }
    }
}

pub fn run<S>(state: S, update: UpdateCb<S>) -> Result<(), String>
where
    S: 'static,
{
    let event_loop = EventLoop::new().unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut runner = Runner {
        state: state,
        update: update,
    };

    event_loop.run_app(&mut runner);

    Ok(())
}

#[inline]
pub(crate) fn get_backend<'a>() -> AtomicRef<'a, WinitBackend> {
    BACKEND.borrow()
}

#[inline]
pub(crate) fn get_mut_backend<'a>() -> AtomicRefMut<'a, WinitBackend> {
    BACKEND.borrow_mut()
}
