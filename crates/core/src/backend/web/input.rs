use super::events::{Event, EventIterator};
use super::utils::{
    canvas_add_event_listener, canvas_position_from_global, document_add_event_listener,
    set_size_dpi, window_add_event_listener,
};
use super::window::WebWindow;
use crate::input::{KeyCode, MouseButton};
use crate::math::{IVec2, Vec2};
use glam::{uvec2, vec2};
use smol_str::SmolStr;
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{Event as WEvent, HtmlCanvasElement, KeyboardEvent, MouseEvent, WheelEvent};

pub(crate) fn add_mouse_listener<F, E>(win: &WebWindow, name: &str, mut handler: F)
where
    E: wasm_bindgen::convert::FromWasmAbi + 'static,
    F: FnMut(&HtmlCanvasElement, &mut EventIterator, E) + 'static,
{
    let events = win.events.clone();
    let canvas = win.canvas.clone();
    let evt = canvas_add_event_listener(&canvas.clone(), name, move |e: E| {
        let mut evts = events.borrow_mut();
        handler(&canvas, &mut evts, e);
    })
    .unwrap();
    std::mem::forget(evt);
}

fn listen_mouse_move(win: &mut WebWindow) {
    let last_pos = Rc::new(RefCell::new(IVec2::ZERO));
    let captured = win.cursor_locked.clone();
    add_mouse_listener(win, "mousemove", move |canvas, events, e: MouseEvent| {
        e.stop_propagation();
        e.prevent_default();
        let old_pos = last_pos.borrow().as_vec2();
        let pos = get_mouse_xy(canvas, e, *captured.borrow(), &mut last_pos.borrow_mut());
        let delta = pos - old_pos;
        events.push(Event::MouseMove { pos, delta });
    });
}

fn listen_mouse_up(win: &mut WebWindow, delayed_dispatch: Rc<RefCell<dyn Fn()>>) {
    add_mouse_listener(win, "mouseup", move |_canvas, events, e: MouseEvent| {
        e.stop_propagation();
        e.prevent_default();
        (*delayed_dispatch.borrow())();
        let btn = mouse_btn_cast(e.button());
        events.push(Event::MouseUp { btn });
    });
}

fn listen_mouse_down(win: &mut WebWindow, delayed_dispatch: Rc<RefCell<dyn Fn()>>) {
    add_mouse_listener(win, "mousedown", move |canvas, events, e: MouseEvent| {
        e.stop_propagation();
        e.prevent_default();
        (*delayed_dispatch.borrow())();
        let _ = canvas.focus();
        let btn = mouse_btn_cast(e.button());
        events.push(Event::MouseDown { btn });
    });
}

fn listen_mouse_leave(win: &mut WebWindow) {
    add_mouse_listener(win, "mouseout", |_canvas, events, e: MouseEvent| {
        e.stop_propagation();
        e.prevent_default();
        events.push(Event::MouseLeave);
    });
}

fn listen_mouse_enter(win: &mut WebWindow) {
    add_mouse_listener(win, "mouseover", |_canvas, events, e: MouseEvent| {
        e.stop_propagation();
        e.prevent_default();
        events.push(Event::MouseEnter);
    });
}

fn listen_wheel(win: &mut WebWindow) {
    let events = win.events.clone();
    let wheel_evt = canvas_add_event_listener(&win.canvas, "wheel", move |e: WheelEvent| {
        e.stop_propagation();
        e.prevent_default();
        let delta = vec2(e.delta_x() as _, e.delta_y() as _) * -1.0;
        events.borrow_mut().push(Event::MouseWheel { delta });
    })
    .unwrap();
    std::mem::forget(wheel_evt);
}

fn listen_cursor_captured(win: &mut WebWindow) {
    let cursor_locked = win.cursor_locked.clone();
    let doc = win.document.clone();
    let canvas = win.canvas.clone();
    let evt = document_add_event_listener("pointerlockchange", move |_: WEvent| {
        match doc.pointer_lock_element() {
            Some(el) => {
                *cursor_locked.borrow_mut() = el.id() == canvas.id();
            }
            _ => {
                *cursor_locked.borrow_mut() = false;
            }
        }
    })
    .unwrap();
    std::mem::forget(evt);
}

fn listen_key_up(win: &mut WebWindow, delayed_dispatch: Rc<RefCell<dyn Fn()>>) {
    add_mouse_listener(win, "keyup", move |_canvas, events, e: KeyboardEvent| {
        (*delayed_dispatch.borrow())();
        let key = keyboard_code_cast(&e.code());
        events.push(Event::KeyUp { key });
    });
}

fn listen_key_down(win: &mut WebWindow, delayed_dispatch: Rc<RefCell<dyn Fn()>>) {
    add_mouse_listener(win, "keydown", move |_canvas, events, e: KeyboardEvent| {
        (*delayed_dispatch.borrow())();
        let key = keyboard_code_cast(&e.code());
        events.push(Event::KeyDown { key });

        // TODO improve this with a hidden input and composition events
        if let Some(text) = text_from_keyboard_event(&e.key()) {
            events.push(Event::CharReceived { text })
        }
    });
}

fn listen_fullscreen_change(win: &mut WebWindow) {
    let def_size = win.config.size;
    let canvas = win.canvas.clone();
    let doc = win.document.clone();
    let events = win.events.clone();
    let last_size = win.fullscreen_last_size.clone();
    let evt = window_add_event_listener("fullscreenchange", move |_: WEvent| {
        let size = if doc.fullscreen() {
            uvec2(canvas.client_width() as _, canvas.client_height() as _)
        } else {
            match *last_size.borrow() {
                Some(size) => size,
                _ => {
                    // fallback to initial size
                    log::error!("Cannot restore size from fullscreen mode.");
                    def_size
                }
            }
        };
        set_size_dpi(&canvas, size.x, size.y);
        events.borrow_mut().push(Event::WindowResize { size });
    });
    std::mem::forget(evt);
}

fn listen_resize_win(win: &mut WebWindow) {
    if !win.config.resizable {
        return;
    }

    let min = win.config.min_size;
    let max = win.config.max_size;
    let canvas = win.canvas.clone();
    let parent = win.parent.clone();
    let events = win.events.clone();

    let evt = window_add_event_listener("resize", move |_: WEvent| {
        let mut parent_size = uvec2(parent.client_width() as _, parent.client_height() as _);
        if parent_size.x == 0 {
            parent_size.x = canvas.client_width() as _;
        }

        if parent_size.y == 0 {
            parent_size.y = canvas.client_height() as _;
        }
        if let Some(min) = min {
            parent_size = parent_size.min(min);
        }

        if let Some(max) = max {
            parent_size = parent_size.max(max);
        }

        set_size_dpi(&canvas, parent_size.x, parent_size.y);
        events
            .borrow_mut()
            .push(Event::WindowResize { size: parent_size });
    });
    std::mem::forget(evt);
}

pub(crate) fn enable_input_events(win: &mut WebWindow) {
    let delayed_dispatch = create_delayed_event_handler(win);

    // mouse and cursor
    listen_mouse_down(win, delayed_dispatch.clone());
    listen_mouse_up(win, delayed_dispatch.clone());
    listen_mouse_move(win);
    listen_mouse_enter(win);
    listen_mouse_leave(win);
    listen_wheel(win);
    listen_cursor_captured(win);

    // keyboard
    listen_key_down(win, delayed_dispatch.clone());
    listen_key_up(win, delayed_dispatch.clone());

    // window events
    listen_resize_win(win);
    listen_fullscreen_change(win);
}

fn get_mouse_xy(
    canvas: &HtmlCanvasElement,
    e: MouseEvent,
    captured: bool,
    last: &mut IVec2,
) -> Vec2 {
    let (x, y) = if captured {
        (last.x + e.movement_x(), last.y + e.movement_y())
    } else {
        canvas_position_from_global(canvas, e)
    };
    last.x = x;
    last.y = y;
    last.as_vec2()
}

fn mouse_btn_cast(btn: i16) -> MouseButton {
    match btn {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        3 => MouseButton::Back,
        4 => MouseButton::Forward,
        _ => MouseButton::Unknown,
    }
}

// Handle events that needs to be executed after an user's action bc browser's security
fn create_delayed_event_handler(win: &mut WebWindow) -> Rc<RefCell<dyn Fn()>> {
    let canvas = win.canvas.clone();
    let doc = win.document.clone();
    let cursor_lock_request = win.cursor_lock_request.clone();
    let fullscreen_request = win.fullscreen_request.clone();
    let last_win_size = win.fullscreen_last_size.clone();
    Rc::new(RefCell::new(move || {
        // Manage cursor lock/unlock requests
        match cursor_lock_request.borrow_mut().take() {
            Some(true) => canvas.request_pointer_lock(),
            Some(false) => doc.exit_pointer_lock(),
            _ => {}
        }

        // Manage fullscreen requests
        match fullscreen_request.borrow_mut().take() {
            Some(true) => {
                let size = uvec2(canvas.client_width() as _, canvas.client_height() as _);
                *last_win_size.borrow_mut() = Some(size);
                if let Err(e) = canvas.request_fullscreen() {
                    log::error!("Error requesting fullscreen mode: {:?}", e);
                }
            }
            Some(false) => doc.exit_fullscreen(),
            _ => {}
        }
    }))
}

pub fn keyboard_code_cast(code: &str) -> KeyCode {
    match code {
        "Backquote" => KeyCode::Backquote,
        "Backslash" => KeyCode::Backslash,
        "BracketLeft" => KeyCode::BracketLeft,
        "BracketRight" => KeyCode::BracketRight,
        "Comma" => KeyCode::Comma,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        "Equal" => KeyCode::Equal,
        "IntlBackslash" => KeyCode::IntlBackslash,
        "IntlRo" => KeyCode::IntlRo,
        "IntlYen" => KeyCode::IntlYen,
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Minus" => KeyCode::Minus,
        "Period" => KeyCode::Period,
        "Quote" => KeyCode::Quote,
        "Semicolon" => KeyCode::Semicolon,
        "Slash" => KeyCode::Slash,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "Backspace" => KeyCode::Backspace,
        "CapsLock" => KeyCode::CapsLock,
        "ContextMenu" => KeyCode::ContextMenu,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "Enter" => KeyCode::Enter,
        "MetaLeft" => KeyCode::SuperLeft,
        "MetaRight" => KeyCode::SuperRight,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "Space" => KeyCode::Space,
        "Tab" => KeyCode::Tab,
        "Convert" => KeyCode::Convert,
        "KanaMode" => KeyCode::KanaMode,
        "Lang1" => KeyCode::Lang1,
        "Lang2" => KeyCode::Lang2,
        "Lang3" => KeyCode::Lang3,
        "Lang4" => KeyCode::Lang4,
        "Lang5" => KeyCode::Lang5,
        "NonConvert" => KeyCode::NonConvert,
        "Delete" => KeyCode::Delete,
        "End" => KeyCode::End,
        "Help" => KeyCode::Help,
        "Home" => KeyCode::Home,
        "Insert" => KeyCode::Insert,
        "PageDown" => KeyCode::PageDown,
        "PageUp" => KeyCode::PageUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "ArrowUp" => KeyCode::ArrowUp,
        "NumLock" => KeyCode::NumLock,
        "Numpad0" => KeyCode::Numpad0,
        "Numpad1" => KeyCode::Numpad1,
        "Numpad2" => KeyCode::Numpad2,
        "Numpad3" => KeyCode::Numpad3,
        "Numpad4" => KeyCode::Numpad4,
        "Numpad5" => KeyCode::Numpad5,
        "Numpad6" => KeyCode::Numpad6,
        "Numpad7" => KeyCode::Numpad7,
        "Numpad8" => KeyCode::Numpad8,
        "Numpad9" => KeyCode::Numpad9,
        "NumpadAdd" => KeyCode::NumpadAdd,
        "NumpadBackspace" => KeyCode::NumpadBackspace,
        "NumpadClear" => KeyCode::NumpadClear,
        "NumpadClearEntry" => KeyCode::NumpadClearEntry,
        "NumpadComma" => KeyCode::NumpadComma,
        "NumpadDecimal" => KeyCode::NumpadDecimal,
        "NumpadDivide" => KeyCode::NumpadDivide,
        "NumpadEnter" => KeyCode::NumpadEnter,
        "NumpadEqual" => KeyCode::NumpadEqual,
        "NumpadHash" => KeyCode::NumpadHash,
        "NumpadMemoryAdd" => KeyCode::NumpadMemoryAdd,
        "NumpadMemoryClear" => KeyCode::NumpadMemoryClear,
        "NumpadMemoryRecall" => KeyCode::NumpadMemoryRecall,
        "NumpadMemoryStore" => KeyCode::NumpadMemoryStore,
        "NumpadMemorySubtract" => KeyCode::NumpadMemorySubtract,
        "NumpadMultiply" => KeyCode::NumpadMultiply,
        "NumpadParenLeft" => KeyCode::NumpadParenLeft,
        "NumpadParenRight" => KeyCode::NumpadParenRight,
        "NumpadStar" => KeyCode::NumpadStar,
        "NumpadSubtract" => KeyCode::NumpadSubtract,
        "Escape" => KeyCode::Escape,
        "Fn" => KeyCode::Fn,
        "FnLock" => KeyCode::FnLock,
        "PrintScreen" => KeyCode::PrintScreen,
        "ScrollLock" => KeyCode::ScrollLock,
        "Pause" => KeyCode::Pause,
        "BrowserBack" => KeyCode::BrowserBack,
        "BrowserFavorites" => KeyCode::BrowserFavorites,
        "BrowserForward" => KeyCode::BrowserForward,
        "BrowserHome" => KeyCode::BrowserHome,
        "BrowserRefresh" => KeyCode::BrowserRefresh,
        "BrowserSearch" => KeyCode::BrowserSearch,
        "BrowserStop" => KeyCode::BrowserStop,
        "Eject" => KeyCode::Eject,
        "LaunchApp1" => KeyCode::LaunchApp1,
        "LaunchApp2" => KeyCode::LaunchApp2,
        "LaunchMail" => KeyCode::LaunchMail,
        "MediaPlayPause" => KeyCode::MediaPlayPause,
        "MediaSelect" => KeyCode::MediaSelect,
        "MediaStop" => KeyCode::MediaStop,
        "MediaTrackNext" => KeyCode::MediaTrackNext,
        "MediaTrackPrevious" => KeyCode::MediaTrackPrevious,
        "Power" => KeyCode::Power,
        "Sleep" => KeyCode::Sleep,
        "AudioVolumeDown" => KeyCode::AudioVolumeDown,
        "AudioVolumeMute" => KeyCode::AudioVolumeMute,
        "AudioVolumeUp" => KeyCode::AudioVolumeUp,
        "WakeUp" => KeyCode::WakeUp,
        "Hyper" => KeyCode::Hyper,
        "Turbo" => KeyCode::Turbo,
        "Abort" => KeyCode::Abort,
        "Resume" => KeyCode::Resume,
        "Suspend" => KeyCode::Suspend,
        "Again" => KeyCode::Again,
        "Copy" => KeyCode::Copy,
        "Cut" => KeyCode::Cut,
        "Find" => KeyCode::Find,
        "Open" => KeyCode::Open,
        "Paste" => KeyCode::Paste,
        "Props" => KeyCode::Props,
        "Select" => KeyCode::Select,
        "Undo" => KeyCode::Undo,
        "Hiragana" => KeyCode::Hiragana,
        "Katakana" => KeyCode::Katakana,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        "F13" => KeyCode::F13,
        "F14" => KeyCode::F14,
        "F15" => KeyCode::F15,
        "F16" => KeyCode::F16,
        "F17" => KeyCode::F17,
        "F18" => KeyCode::F18,
        "F19" => KeyCode::F19,
        "F20" => KeyCode::F20,
        "F21" => KeyCode::F21,
        "F22" => KeyCode::F22,
        "F23" => KeyCode::F23,
        "F24" => KeyCode::F24,
        "F25" => KeyCode::F25,
        "F26" => KeyCode::F26,
        "F27" => KeyCode::F27,
        "F28" => KeyCode::F28,
        "F29" => KeyCode::F29,
        "F30" => KeyCode::F30,
        "F31" => KeyCode::F31,
        "F32" => KeyCode::F32,
        "F33" => KeyCode::F33,
        "F34" => KeyCode::F34,
        "F35" => KeyCode::F35,
        _ => KeyCode::Unknown,
    }
}

fn is_fn_key(key: &str) -> bool {
    key.starts_with('F') && key.len() >= 2
}

const CONTROL_KEYS: [&str; 25] = [
    "Alt",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "Backspace",
    "CapsLock",
    "ContextMenu",
    "Control",
    "Dead",
    "Delete",
    "End",
    "Esc",
    "Escape",
    "GroupNext",
    "Help",
    "Home",
    "Insert",
    "Meta",
    "NumLock",
    "PageDown",
    "PageUp",
    "Pause",
    "ScrollLock",
    "Shift",
];

fn is_ctrl_key(key: &str) -> bool {
    CONTROL_KEYS.contains(&key)
}

fn text_from_keyboard_event(key: &str) -> Option<SmolStr> {
    if is_fn_key(key) || is_ctrl_key(key) {
        return None;
    }

    Some(match key {
        "Enter" => SmolStr::new("\r"),
        "Tab" => SmolStr::new("\t"),
        _ => SmolStr::new(key),
    })
}
