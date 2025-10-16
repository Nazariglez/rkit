use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{HtmlCanvasElement, Window};

#[inline]
pub(crate) fn set_cursor_visible(canvas: &HtmlCanvasElement, visible: bool) {
    let mode = if visible { "" } else { "none" };

    if let Err(e) = canvas.style().set_property("cursor", mode) {
        log::error!("{e:?}");
    }
}

pub(crate) fn set_size_dpi(canvas: &HtmlCanvasElement, width: u32, height: u32) {
    let auto_res = canvas
        .get_attribute("gk-auto-res")
        .unwrap_or_else(|| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
    let dpi = if auto_res {
        web_sys::window().unwrap().device_pixel_ratio() as f32
    } else {
        1.0
    };

    let ww = width as f32 * dpi;
    let hh = height as f32 * dpi;

    canvas.set_width(ww as _);
    canvas.set_height(hh as _);

    if let Err(e) = canvas.style().set_property("width", &format!("{width}px")) {
        log::error!("{e:?}");
    }

    if let Err(e) = canvas
        .style()
        .set_property("height", &format!("{height}px"))
    {
        log::error!("{e:?}");
    }

    if let Err(e) = canvas.set_attribute("gk-width", &width.to_string()) {
        log::error!("{e:?}");
    }

    if let Err(e) = canvas.set_attribute("gk-height", &height.to_string()) {
        log::error!("{e:?}");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameId {
    Raf(i32),
    Timeout(i32),
}

#[inline]
pub(crate) fn request_next_frame(
    win: &Window,
    offscreen_dt: Option<f32>,
    f: &Closure<dyn FnMut()>,
) -> FrameId {
    let use_timeout = !is_win_visible(win);
    match offscreen_dt {
        Some(dt) if use_timeout => FrameId::Timeout(set_timeout(win, dt * 1000.0, f)),
        _ => FrameId::Raf(request_animation_frame(win, f)),
    }
}

#[inline]
pub(crate) fn cancel_frame(win: &Window, id: FrameId) {
    match id {
        FrameId::Raf(id) => {
            let _ = win.cancel_animation_frame(id);
        }
        FrameId::Timeout(id) => {
            win.clear_timeout_with_handle(id);
        }
    }
}

#[inline]
pub(crate) fn is_win_visible(win: &Window) -> bool {
    !win.document().unwrap().hidden()
}

#[inline]
pub(crate) fn request_animation_frame(win: &Window, f: &Closure<dyn FnMut()>) -> i32 {
    win.request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK")
}

#[inline]
pub(crate) fn set_timeout(win: &Window, dt: f32, f: &Closure<dyn FnMut()>) -> i32 {
    win.set_timeout_with_callback_and_timeout_and_arguments_0(f.as_ref().unchecked_ref(), dt as _)
        .expect("should register `setTimeout` OK")
}

pub(crate) fn canvas_add_event_listener<F, E>(
    canvas: &HtmlCanvasElement,
    name: &str,
    handler: F,
) -> Result<Closure<dyn FnMut(E)>, String>
where
    E: wasm_bindgen::convert::FromWasmAbi + 'static,
    F: FnMut(E) + 'static,
{
    let mut handler = handler;
    let closure = Closure::wrap(Box::new(move |e: E| {
        handler(e);
    }) as Box<dyn FnMut(_)>);

    canvas
        .add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())
        .map_err(|_| format!("Invalid event name: {name}"))?;
    Ok(closure)
}

pub(crate) fn window_add_event_listener<F, E>(
    name: &str,
    handler: F,
) -> Result<Closure<dyn FnMut(E)>, String>
where
    E: wasm_bindgen::convert::FromWasmAbi + 'static,
    F: FnMut(E) + 'static,
{
    let win = web_sys::window().ok_or_else(|| "global window doesn't exists".to_string())?;

    let mut handler = handler;
    let closure = Closure::wrap(Box::new(move |e: E| {
        handler(e);
    }) as Box<dyn FnMut(_)>);

    win.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())
        .map_err(|_| format!("Invalid event name: {name}"))?;
    Ok(closure)
}

pub(crate) fn document_add_event_listener<F, E>(
    name: &str,
    handler: F,
) -> Result<Closure<dyn FnMut(E)>, String>
where
    E: wasm_bindgen::convert::FromWasmAbi + 'static,
    F: FnMut(E) + 'static,
{
    let doc = web_sys::window()
        .ok_or_else(|| "global window doesn't exists".to_string())?
        .document()
        .ok_or("Can't access document dom object ")?;

    let mut handler = handler;
    let closure = Closure::wrap(Box::new(move |e: E| {
        handler(e);
    }) as Box<dyn FnMut(_)>);

    doc.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())
        .map_err(|_| format!("Invalid event name: {name}"))?;
    Ok(closure)
}

#[inline]
pub(crate) fn canvas_position_from_global(
    canvas: &HtmlCanvasElement,
    evt: web_sys::MouseEvent,
) -> (i32, i32) {
    let (x, y) = canvas_pos(canvas, evt.client_x(), evt.client_y());
    (x as _, y as _)
}

#[inline]
fn canvas_pos(canvas: &HtmlCanvasElement, client_x: i32, client_y: i32) -> (f32, f32) {
    let client_x = client_x as f32;
    let client_y = client_y as f32;
    let rect = canvas.get_bounding_client_rect();
    let x = client_x - rect.left() as f32;
    let y = client_y - rect.top() as f32;
    (x, y)
}

pub(crate) fn get_gk_size(canvas: &HtmlCanvasElement) -> (u32, u32) {
    let width = canvas
        .get_attribute("gk-width")
        .unwrap_or_else(|| "0".to_string())
        .parse::<u32>()
        .unwrap_or(0);

    let height = canvas
        .get_attribute("gk-height")
        .unwrap_or_else(|| "0".to_string())
        .parse::<u32>()
        .unwrap_or(0);

    (width, height)
}
