use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys;
use winit::window::{Window, WindowId};

#[derive(Default)]
pub(crate) struct WinitApp {
    window: Option<Window>,
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut attrs =
            Window::default_attributes().with_inner_size(LogicalSize::new(800.0, 600.0));

        #[cfg(target_arch = "wasm32")]
        {
            attrs = attrs.with_append(true);
        }
        self.window = Some(event_loop.create_window(attrs).unwrap());
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

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

pub fn init() -> Result<(), String> {
    let event_loop = EventLoop::new().unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = WinitApp::default();
    event_loop.run_app(&mut app);

    Ok(())
}

pub fn set_title(title: &str) {}

pub fn title(title: &str) -> &str {
    "todo"
}
