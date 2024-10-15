#![cfg(not(target_arch = "wasm32"))]

use crate::math::uvec2;
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use once_cell::sync::Lazy;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, Ime, MouseButton as WMouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode as WKeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowAttributes, WindowId};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys;

use super::backend::{BackendImpl, GfxBackendImpl};
use crate::app::WindowConfig;
use crate::backend::wgpu::GfxBackend;
use crate::builder::AppBuilder;
use crate::input::{KeyCode, KeyboardState, MouseButton, MouseState};
use crate::math::{vec2, Vec2};
// TODO, screen_size, positions etc... must be logical or physical pixels?

#[cfg(feature = "gamepad")]
use crate::backend::gamepad_gilrs::GilrsBackend;
use crate::events::{CoreEvent, CORE_EVENTS_MAP};
#[cfg(feature = "gamepad")]
use crate::input::GamepadState;
use crate::time;

pub(crate) static BACKEND: Lazy<AtomicRefCell<WinitBackend>> =
    Lazy::new(|| AtomicRefCell::new(WinitBackend::default()));

#[derive(Default)]
pub(crate) struct WinitBackend {
    win_opts: WindowAttributes,
    window: Option<Window>,
    request_close: bool,
    mouse_state: MouseState,
    keyboard_state: KeyboardState,

    cursor_locked: bool,
    cursor_visible: bool,

    #[cfg(feature = "gamepad")]
    gilrs: GilrsBackend,
    gfx: Option<GfxBackend>,
}

impl BackendImpl<GfxBackend> for WinitBackend {
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
    fn set_min_size(&mut self, size: Vec2) {
        debug_assert!(self.window.is_some(), "Window must be present");
        let _ = self
            .window
            .as_mut()
            .unwrap()
            .set_min_inner_size(Some(LogicalSize::new(size.x, size.y)));
    }

    #[inline]
    fn set_max_size(&mut self, size: Vec2) {
        debug_assert!(self.window.is_some(), "Window must be present");
        let _ = self
            .window
            .as_mut()
            .unwrap()
            .set_max_inner_size(Some(LogicalSize::new(size.x, size.y)));
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

    #[inline]
    fn close(&mut self) {
        debug_assert!(self.window.is_some(), "Window must be present");
        self.request_close = true;
    }

    #[inline]
    fn mouse_state(&self) -> &MouseState {
        debug_assert!(self.window.is_some(), "Window must be present");
        &self.mouse_state
    }

    fn set_cursor_lock(&mut self, lock: bool) {
        if self.cursor_locked == lock {
            return;
        }

        let is_macos = cfg!(target_os = "macos");
        if is_macos {
            log::warn!("Cursor Lock is not implemented on MacOS yet.");
            return;
        }

        let mode = if lock {
            CursorGrabMode::Confined
        } else {
            CursorGrabMode::None
        };

        let res = self.window.as_mut().unwrap().set_cursor_grab(mode);
        if let Err(err) = res {
            log::warn!("Error locking cursor: {}", err.to_string());
            return;
        }

        self.cursor_locked = lock;
    }

    fn is_cursor_locked(&self) -> bool {
        self.cursor_locked
    }

    fn set_cursor_visible(&mut self, visible: bool) {
        if self.cursor_visible == visible {
            return;
        }

        self.window.as_mut().unwrap().set_cursor_visible(visible);
        self.cursor_visible = visible;
    }

    fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    #[inline]
    fn keyboard_state(&self) -> &KeyboardState {
        debug_assert!(self.window.is_some(), "Window must be present");
        &self.keyboard_state
    }

    #[cfg(feature = "gamepad")]
    #[inline]
    fn gamepad_state(&self) -> &GamepadState {
        &self.gilrs.state
    }

    #[inline]
    fn gfx(&mut self) -> &mut GfxBackend {
        self.gfx.as_mut().unwrap()
    }
}

struct Runner<S> {
    window_attrs: WindowAttributes,
    init: Option<Box<dyn FnOnce() -> S>>,
    state: Option<S>,
    update: Box<dyn FnMut(&mut S)>,
    vsync: bool,
}

impl<S> ApplicationHandler for Runner<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut attrs = self.window_attrs.clone();

        #[cfg(target_arch = "wasm32")]
        {
            attrs = attrs.with_append(true);
        }

        let win = event_loop.create_window(attrs).unwrap();
        win.set_ime_allowed(true); // allow for chars

        let win_size = win.inner_size();
        let gfx_initiated = get_backend().gfx.is_some();
        if gfx_initiated {
            let res = pollster::block_on(get_mut_backend().gfx.as_mut().unwrap().update_surface(
                &win,
                self.vsync,
                uvec2(win_size.width, win_size.height),
            ));
            match res {
                Ok(_) => {
                    log::trace!("Surface updated");
                }
                Err(e) => {
                    log::error!("Error updating surface on Gfx backend: {}", e);
                }
            }
        } else {
            let gfx = pollster::block_on(GfxBackend::init(
                &win,
                self.vsync,
                uvec2(win_size.width, win_size.height),
            ));
            match gfx {
                Ok(gfx) => {
                    get_mut_backend().gfx = Some(gfx);
                    log::trace!("Surface initiated");
                }
                Err(e) => {
                    log::error!("Error initiating Gfx backend: {}", e);
                }
            }
        }

        win.request_redraw();
        get_mut_backend().window = Some(win);
        if let Some(init_cb) = self.init.take() {
            self.state = Some(init_cb());
        }
        CORE_EVENTS_MAP.borrow().trigger(CoreEvent::Init);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let mut bck = get_mut_backend();
                let old_mouse_pos = bck.mouse_state.position();

                let w_pos = position.to_logical::<f32>(bck.window.as_ref().unwrap().scale_factor());
                let pos = vec2(w_pos.x, w_pos.y);
                let motion_delta = pos - old_mouse_pos;

                bck.mouse_state.position = pos;
                bck.mouse_state.motion_delta = motion_delta;
                if motion_delta.x != 0.0 || motion_delta.y != 0.0 {
                    bck.mouse_state.moving = true;
                }
                bck.mouse_state.cursor_on_screen = true;
            }
            WindowEvent::CursorEntered { .. } => {
                get_mut_backend().mouse_state.cursor_on_screen = true;
            }
            WindowEvent::CursorLeft { .. } => {
                get_mut_backend().mouse_state.cursor_on_screen = false;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let mut bck = get_mut_backend();
                let scale_factor = bck.window.as_ref().unwrap().scale_factor() as f32;
                let value = match delta {
                    MouseScrollDelta::LineDelta(x, y) => vec2(x, y) * 50.0,
                    MouseScrollDelta::PixelDelta(dt) => {
                        let delta =
                            dt.to_logical::<f32>(bck.window.as_ref().unwrap().scale_factor());
                        vec2(delta.x, delta.y)
                    }
                };
                bck.mouse_state.wheel_delta = value;
                bck.mouse_state.scrolling = true;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = mouse_btn_cast(button);
                let mut bck = get_mut_backend();
                match state {
                    ElementState::Pressed => bck.mouse_state.press(btn),
                    ElementState::Released => bck.mouse_state.release(btn),
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key = physical_key_cast(event.physical_key);
                let mut bck = get_mut_backend();
                match event.state {
                    ElementState::Pressed => bck.keyboard_state.press(key),
                    ElementState::Released => bck.keyboard_state.release(key),
                }

                // chars
                if let Some(txt) = event.text {
                    bck.keyboard_state.add_text(txt.as_str());
                }
            }
            WindowEvent::Ime(ime) => {
                // chars
                match ime {
                    Ime::Commit(c) => {
                        get_mut_backend().keyboard_state.add_text(c.as_str());
                    }
                    _ => {}
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                time::tick();

                #[cfg(feature = "gamepad")]
                {
                    // gamepad must be updated before the update cb
                    get_mut_backend().gilrs.tick();
                }

                // app's update cb
                CORE_EVENTS_MAP.borrow().trigger(CoreEvent::PreUpdate);

                get_mut_backend().gfx().prepare_frame();
                (*self.update)(self.state.as_mut().unwrap());
                get_mut_backend().gfx().present_frame();

                CORE_EVENTS_MAP.borrow().trigger(CoreEvent::PostUpdate);

                // post-update
                let mut bck = get_mut_backend();
                bck.window.as_ref().unwrap().request_redraw();
                bck.mouse_state.tick();
                bck.keyboard_state.tick();
            }
            WindowEvent::Resized(size) => {
                let mut bck = get_mut_backend();
                bck.gfx.as_mut().unwrap().resize(size.width, size.height);
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer,
            } => {
                // println!("scale factor: {scale_factor:?} size:{inner_size_writer:?}");
            }
            _ => (),
        }

        let request_close = get_backend().request_close;
        if request_close {
            event_loop.exit();
        }
    }
}

pub fn run<S>(builder: AppBuilder<S>) -> Result<(), String>
where
    S: 'static,
{
    let AppBuilder {
        window,
        init_cb,
        update_cb,
        cleanup_cb,
    } = builder;

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let vsync = window.vsync;

    let mut runner = Runner {
        window_attrs: window_attrs(window),
        init: Some(init_cb),
        state: None,
        update: update_cb,
        vsync,
    };

    event_loop.run_app(&mut runner).map_err(|e| e.to_string())?;

    // at this point the runner is not in use, the app is closing
    cleanup_cb(runner.state.as_mut().unwrap());

    CORE_EVENTS_MAP.borrow().trigger(CoreEvent::CleanUp);

    Ok(())
}

#[inline]
pub(crate) fn get_backend() -> AtomicRef<'static, WinitBackend> {
    BACKEND.borrow()
}

#[inline]
pub(crate) fn get_mut_backend() -> AtomicRefMut<'static, WinitBackend> {
    BACKEND.borrow_mut()
}

fn window_attrs(config: WindowConfig) -> WindowAttributes {
    let WindowConfig {
        title,
        size,
        min_size,
        max_size,
        resizable,
        vsync: _,
    } = config;

    let mut attrs = WindowAttributes::default()
        .with_title(title)
        .with_inner_size(LogicalSize::new(size.x, size.y))
        .with_resizable(resizable);

    if let Some(ms) = min_size {
        attrs = attrs.with_min_inner_size(LogicalSize::new(ms.x, ms.y));
    }

    if let Some(ms) = max_size {
        attrs = attrs.with_max_inner_size(LogicalSize::new(ms.x, ms.y));
    }

    attrs
}

fn mouse_btn_cast(wbtn: WMouseButton) -> MouseButton {
    match wbtn {
        WMouseButton::Left => MouseButton::Left,
        WMouseButton::Right => MouseButton::Right,
        WMouseButton::Middle => MouseButton::Middle,
        WMouseButton::Back => MouseButton::Back,
        WMouseButton::Forward => MouseButton::Forward,
        WMouseButton::Other(_) => MouseButton::Unknown,
    }
}

fn physical_key_cast(wkey: PhysicalKey) -> KeyCode {
    match wkey {
        PhysicalKey::Code(code) => match code {
            WKeyCode::Backquote => KeyCode::Backquote,
            WKeyCode::Backslash => KeyCode::Backslash,
            WKeyCode::BracketLeft => KeyCode::BracketLeft,
            WKeyCode::BracketRight => KeyCode::BracketRight,
            WKeyCode::Comma => KeyCode::Comma,
            WKeyCode::Digit0 => KeyCode::Digit0,
            WKeyCode::Digit1 => KeyCode::Digit1,
            WKeyCode::Digit2 => KeyCode::Digit2,
            WKeyCode::Digit3 => KeyCode::Digit3,
            WKeyCode::Digit4 => KeyCode::Digit4,
            WKeyCode::Digit5 => KeyCode::Digit5,
            WKeyCode::Digit6 => KeyCode::Digit6,
            WKeyCode::Digit7 => KeyCode::Digit7,
            WKeyCode::Digit8 => KeyCode::Digit8,
            WKeyCode::Digit9 => KeyCode::Digit9,
            WKeyCode::Equal => KeyCode::Equal,
            WKeyCode::IntlBackslash => KeyCode::IntlBackslash,
            WKeyCode::IntlRo => KeyCode::IntlRo,
            WKeyCode::IntlYen => KeyCode::IntlYen,
            WKeyCode::KeyA => KeyCode::KeyA,
            WKeyCode::KeyB => KeyCode::KeyB,
            WKeyCode::KeyC => KeyCode::KeyC,
            WKeyCode::KeyD => KeyCode::KeyD,
            WKeyCode::KeyE => KeyCode::KeyE,
            WKeyCode::KeyF => KeyCode::KeyF,
            WKeyCode::KeyG => KeyCode::KeyG,
            WKeyCode::KeyH => KeyCode::KeyH,
            WKeyCode::KeyI => KeyCode::KeyI,
            WKeyCode::KeyJ => KeyCode::KeyJ,
            WKeyCode::KeyK => KeyCode::KeyK,
            WKeyCode::KeyL => KeyCode::KeyL,
            WKeyCode::KeyM => KeyCode::KeyM,
            WKeyCode::KeyN => KeyCode::KeyN,
            WKeyCode::KeyO => KeyCode::KeyO,
            WKeyCode::KeyP => KeyCode::KeyP,
            WKeyCode::KeyQ => KeyCode::KeyQ,
            WKeyCode::KeyR => KeyCode::KeyR,
            WKeyCode::KeyS => KeyCode::KeyS,
            WKeyCode::KeyT => KeyCode::KeyT,
            WKeyCode::KeyU => KeyCode::KeyU,
            WKeyCode::KeyV => KeyCode::KeyV,
            WKeyCode::KeyW => KeyCode::KeyW,
            WKeyCode::KeyX => KeyCode::KeyX,
            WKeyCode::KeyY => KeyCode::KeyY,
            WKeyCode::KeyZ => KeyCode::KeyZ,
            WKeyCode::Minus => KeyCode::Minus,
            WKeyCode::Period => KeyCode::Period,
            WKeyCode::Quote => KeyCode::Quote,
            WKeyCode::Semicolon => KeyCode::Semicolon,
            WKeyCode::Slash => KeyCode::Slash,
            WKeyCode::AltLeft => KeyCode::AltLeft,
            WKeyCode::AltRight => KeyCode::AltRight,
            WKeyCode::Backspace => KeyCode::Backspace,
            WKeyCode::CapsLock => KeyCode::CapsLock,
            WKeyCode::ContextMenu => KeyCode::ContextMenu,
            WKeyCode::ControlLeft => KeyCode::ControlLeft,
            WKeyCode::ControlRight => KeyCode::ControlRight,
            WKeyCode::Enter => KeyCode::Enter,
            WKeyCode::SuperLeft => KeyCode::SuperLeft,
            WKeyCode::SuperRight => KeyCode::SuperRight,
            WKeyCode::ShiftLeft => KeyCode::ShiftLeft,
            WKeyCode::ShiftRight => KeyCode::ShiftRight,
            WKeyCode::Space => KeyCode::Space,
            WKeyCode::Tab => KeyCode::Tab,
            WKeyCode::Convert => KeyCode::Convert,
            WKeyCode::KanaMode => KeyCode::KanaMode,
            WKeyCode::Lang1 => KeyCode::Lang1,
            WKeyCode::Lang2 => KeyCode::Lang2,
            WKeyCode::Lang3 => KeyCode::Lang3,
            WKeyCode::Lang4 => KeyCode::Lang4,
            WKeyCode::Lang5 => KeyCode::Lang5,
            WKeyCode::NonConvert => KeyCode::NonConvert,
            WKeyCode::Delete => KeyCode::Delete,
            WKeyCode::End => KeyCode::End,
            WKeyCode::Help => KeyCode::Help,
            WKeyCode::Home => KeyCode::Home,
            WKeyCode::Insert => KeyCode::Insert,
            WKeyCode::PageDown => KeyCode::PageDown,
            WKeyCode::PageUp => KeyCode::PageUp,
            WKeyCode::ArrowDown => KeyCode::ArrowDown,
            WKeyCode::ArrowLeft => KeyCode::ArrowLeft,
            WKeyCode::ArrowRight => KeyCode::ArrowRight,
            WKeyCode::ArrowUp => KeyCode::ArrowUp,
            WKeyCode::NumLock => KeyCode::NumLock,
            WKeyCode::Numpad0 => KeyCode::Numpad0,
            WKeyCode::Numpad1 => KeyCode::Numpad1,
            WKeyCode::Numpad2 => KeyCode::Numpad2,
            WKeyCode::Numpad3 => KeyCode::Numpad3,
            WKeyCode::Numpad4 => KeyCode::Numpad4,
            WKeyCode::Numpad5 => KeyCode::Numpad5,
            WKeyCode::Numpad6 => KeyCode::Numpad6,
            WKeyCode::Numpad7 => KeyCode::Numpad7,
            WKeyCode::Numpad8 => KeyCode::Numpad8,
            WKeyCode::Numpad9 => KeyCode::Numpad9,
            WKeyCode::NumpadAdd => KeyCode::NumpadAdd,
            WKeyCode::NumpadBackspace => KeyCode::NumpadBackspace,
            WKeyCode::NumpadClear => KeyCode::NumpadClear,
            WKeyCode::NumpadClearEntry => KeyCode::NumpadClearEntry,
            WKeyCode::NumpadComma => KeyCode::NumpadComma,
            WKeyCode::NumpadDecimal => KeyCode::NumpadDecimal,
            WKeyCode::NumpadDivide => KeyCode::NumpadDivide,
            WKeyCode::NumpadEnter => KeyCode::NumpadEnter,
            WKeyCode::NumpadEqual => KeyCode::NumpadEqual,
            WKeyCode::NumpadHash => KeyCode::NumpadHash,
            WKeyCode::NumpadMemoryAdd => KeyCode::NumpadMemoryAdd,
            WKeyCode::NumpadMemoryClear => KeyCode::NumpadMemoryClear,
            WKeyCode::NumpadMemoryRecall => KeyCode::NumpadMemoryRecall,
            WKeyCode::NumpadMemoryStore => KeyCode::NumpadMemoryStore,
            WKeyCode::NumpadMemorySubtract => KeyCode::NumpadMemorySubtract,
            WKeyCode::NumpadMultiply => KeyCode::NumpadMultiply,
            WKeyCode::NumpadParenLeft => KeyCode::NumpadParenLeft,
            WKeyCode::NumpadParenRight => KeyCode::NumpadParenRight,
            WKeyCode::NumpadStar => KeyCode::NumpadStar,
            WKeyCode::NumpadSubtract => KeyCode::NumpadSubtract,
            WKeyCode::Escape => KeyCode::Escape,
            WKeyCode::Fn => KeyCode::Fn,
            WKeyCode::FnLock => KeyCode::FnLock,
            WKeyCode::PrintScreen => KeyCode::PrintScreen,
            WKeyCode::ScrollLock => KeyCode::ScrollLock,
            WKeyCode::Pause => KeyCode::Pause,
            WKeyCode::BrowserBack => KeyCode::BrowserBack,
            WKeyCode::BrowserFavorites => KeyCode::BrowserFavorites,
            WKeyCode::BrowserForward => KeyCode::BrowserForward,
            WKeyCode::BrowserHome => KeyCode::BrowserHome,
            WKeyCode::BrowserRefresh => KeyCode::BrowserRefresh,
            WKeyCode::BrowserSearch => KeyCode::BrowserSearch,
            WKeyCode::BrowserStop => KeyCode::BrowserStop,
            WKeyCode::Eject => KeyCode::Eject,
            WKeyCode::LaunchApp1 => KeyCode::LaunchApp1,
            WKeyCode::LaunchApp2 => KeyCode::LaunchApp2,
            WKeyCode::LaunchMail => KeyCode::LaunchMail,
            WKeyCode::MediaPlayPause => KeyCode::MediaPlayPause,
            WKeyCode::MediaSelect => KeyCode::MediaSelect,
            WKeyCode::MediaStop => KeyCode::MediaStop,
            WKeyCode::MediaTrackNext => KeyCode::MediaTrackNext,
            WKeyCode::MediaTrackPrevious => KeyCode::MediaTrackPrevious,
            WKeyCode::Power => KeyCode::Power,
            WKeyCode::Sleep => KeyCode::Sleep,
            WKeyCode::AudioVolumeDown => KeyCode::AudioVolumeDown,
            WKeyCode::AudioVolumeMute => KeyCode::AudioVolumeMute,
            WKeyCode::AudioVolumeUp => KeyCode::AudioVolumeUp,
            WKeyCode::WakeUp => KeyCode::WakeUp,
            WKeyCode::Meta => KeyCode::Meta,
            WKeyCode::Hyper => KeyCode::Hyper,
            WKeyCode::Turbo => KeyCode::Turbo,
            WKeyCode::Abort => KeyCode::Abort,
            WKeyCode::Resume => KeyCode::Resume,
            WKeyCode::Suspend => KeyCode::Suspend,
            WKeyCode::Again => KeyCode::Again,
            WKeyCode::Copy => KeyCode::Copy,
            WKeyCode::Cut => KeyCode::Cut,
            WKeyCode::Find => KeyCode::Find,
            WKeyCode::Open => KeyCode::Open,
            WKeyCode::Paste => KeyCode::Paste,
            WKeyCode::Props => KeyCode::Props,
            WKeyCode::Select => KeyCode::Select,
            WKeyCode::Undo => KeyCode::Undo,
            WKeyCode::Hiragana => KeyCode::Hiragana,
            WKeyCode::Katakana => KeyCode::Katakana,
            WKeyCode::F1 => KeyCode::F1,
            WKeyCode::F2 => KeyCode::F2,
            WKeyCode::F3 => KeyCode::F3,
            WKeyCode::F4 => KeyCode::F4,
            WKeyCode::F5 => KeyCode::F5,
            WKeyCode::F6 => KeyCode::F6,
            WKeyCode::F7 => KeyCode::F7,
            WKeyCode::F8 => KeyCode::F8,
            WKeyCode::F9 => KeyCode::F9,
            WKeyCode::F10 => KeyCode::F10,
            WKeyCode::F11 => KeyCode::F11,
            WKeyCode::F12 => KeyCode::F12,
            WKeyCode::F13 => KeyCode::F13,
            WKeyCode::F14 => KeyCode::F14,
            WKeyCode::F15 => KeyCode::F15,
            WKeyCode::F16 => KeyCode::F16,
            WKeyCode::F17 => KeyCode::F17,
            WKeyCode::F18 => KeyCode::F18,
            WKeyCode::F19 => KeyCode::F19,
            WKeyCode::F20 => KeyCode::F20,
            WKeyCode::F21 => KeyCode::F21,
            WKeyCode::F22 => KeyCode::F22,
            WKeyCode::F23 => KeyCode::F23,
            WKeyCode::F24 => KeyCode::F24,
            WKeyCode::F25 => KeyCode::F25,
            WKeyCode::F26 => KeyCode::F26,
            WKeyCode::F27 => KeyCode::F27,
            WKeyCode::F28 => KeyCode::F28,
            WKeyCode::F29 => KeyCode::F29,
            WKeyCode::F30 => KeyCode::F30,
            WKeyCode::F31 => KeyCode::F31,
            WKeyCode::F32 => KeyCode::F32,
            WKeyCode::F33 => KeyCode::F33,
            WKeyCode::F34 => KeyCode::F34,
            WKeyCode::F35 => KeyCode::F35,
            _ => KeyCode::Unknown,
        },
        PhysicalKey::Unidentified(_) => KeyCode::Unknown,
    }
}
