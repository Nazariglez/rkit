use super::events::{Event, EventIterator};
use super::utils::{canvas_add_event_listener, canvas_position_from_global};
use super::window::WebWindow;
use crate::input::MouseButton;
use crate::math::{IVec2, Vec2};
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{HtmlCanvasElement, MouseEvent};

pub(crate) fn add_mouse_listener<F, E>(
    win: &WebWindow,
    name: &str,
    last_pos: Rc<RefCell<IVec2>>,
    mut handler: F,
) where
    E: wasm_bindgen::convert::FromWasmAbi + 'static,
    F: FnMut(&HtmlCanvasElement, &mut EventIterator, &mut IVec2, E) + 'static,
{
    let events = win.events.clone();
    let canvas = win.canvas.clone();
    let evt = canvas_add_event_listener(&canvas.clone(), name, move |e: E| {
        let mut evts = events.borrow_mut();
        let mut pos = last_pos.borrow_mut();
        handler(&canvas, &mut evts, &mut pos, e);
    });
    std::mem::forget(evt);
}

pub(crate) fn enable_mouse(
    win: &mut WebWindow, /*, fullscreen_dispatcher: Rc<RefCell<dyn Fn()>>*/
) {
    let last_pos = Rc::new(RefCell::new(IVec2::ZERO));

    add_mouse_listener(
        win,
        "mousemove",
        last_pos.clone(),
        |canvas, events, last_pos, e: MouseEvent| {
            e.stop_propagation();
            e.prevent_default();
            let pos = get_mouse_xy(
                &canvas, e,     // *captured.borrow(),
                false, // TODO captured mouse
                last_pos,
            );
            events.push(Event::MouseMove { pos });
        },
    );

    add_mouse_listener(
        win,
        "mouseup",
        last_pos.clone(),
        |canvas, events, last_pos, e: MouseEvent| {
            e.stop_propagation();
            e.prevent_default();
            let btn = mouse_btn_cast(e.button());
            let pos = get_mouse_xy(
                &canvas, e,     // *captured.borrow(),
                false, // TODO captured mouse
                last_pos,
            );
            canvas.focus();
            events.push(Event::MouseUp { btn, pos });
        },
    );

    add_mouse_listener(
        win,
        "mousedown",
        last_pos.clone(),
        |canvas, events, last_pos, e: MouseEvent| {
            e.stop_propagation();
            e.prevent_default();
            let btn = mouse_btn_cast(e.button());
            let pos = get_mouse_xy(
                &canvas, e,     // *captured.borrow(),
                false, // TODO captured mouse
                last_pos,
            );
            canvas.focus();
            events.push(Event::MouseDown { btn, pos });
        },
    );
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
