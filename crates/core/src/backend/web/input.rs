use super::events::{Event, EventIterator};
use super::utils::{
    canvas_add_event_listener, canvas_position_from_global, document_add_event_listener,
};
use super::window::WebWindow;
use crate::input::MouseButton;
use crate::math::{IVec2, Vec2};
use glam::vec2;
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{Event as WEvent, HtmlCanvasElement, MouseEvent, WheelEvent};

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
        let pos = get_mouse_xy(&canvas, e, *captured.borrow(), &mut last_pos.borrow_mut());
        let delta = pos - old_pos;
        events.push(Event::MouseMove { pos, delta });
    });
}

fn listen_mouse_up(win: &mut WebWindow, delayed_dispatch: Rc<RefCell<dyn Fn()>>) {
    add_mouse_listener(win, "mouseup", move |canvas, events, e: MouseEvent| {
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
        canvas.focus();
        let btn = mouse_btn_cast(e.button());
        events.push(Event::MouseDown { btn });
    });
}

fn listen_mouse_leave(win: &mut WebWindow) {
    add_mouse_listener(win, "mouseout", |canvas, events, e: MouseEvent| {
        e.stop_propagation();
        e.prevent_default();
        events.push(Event::MouseLeave);
    });
}

fn listen_mouse_enter(win: &mut WebWindow) {
    add_mouse_listener(win, "mouseover", |canvas, events, e: MouseEvent| {
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
    Rc::new(RefCell::new(move || {
        match cursor_lock_request.borrow_mut().take() {
            Some(true) => canvas.request_pointer_lock(),
            Some(false) => doc.exit_pointer_lock(),
            _ => {}
        }
    }))
}
