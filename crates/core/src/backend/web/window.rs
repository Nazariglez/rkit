use super::utils::{canvas_add_event_listener, set_size_dpi};
use crate::app::WindowConfig;
use crate::math::{uvec2, UVec2};
use js_sys::wasm_bindgen::JsValue;
use raw_window_handle::{
    DisplayHandle, HasDisplayHandle, RawDisplayHandle, RawWindowHandle, WebCanvasWindowHandle,
    WebDisplayHandle,
};
use std::ptr::NonNull;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event as WebEvent, HtmlCanvasElement, Window};
use wgpu::rwh::{HandleError, HasWindowHandle, WebWindowHandle, WindowHandle};

pub(crate) struct WebWindow {
    pub canvas: HtmlCanvasElement,
    pub win: Window,
    pub document: Document,
    pub parent: Element,
    pub dpi: f32,

    pub config: WindowConfig,
}

impl HasWindowHandle for WebWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let canvas: &JsValue = &self.canvas;
        let window_handle = WebCanvasWindowHandle::new(NonNull::from(canvas).cast());
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::WebCanvas(window_handle)) })
    }
}

impl HasDisplayHandle for WebWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Web(WebDisplayHandle::new())) })
    }
}

impl WebWindow {
    pub fn new(config: WindowConfig) -> Result<Self, String> {
        let window =
            web_sys::window().ok_or_else(|| String::from("Can't access window dom object."))?;
        let document = window
            .document()
            .ok_or("Can't access document dom object ")?;

        let canvas = get_or_create_canvas(&document, "gk_canvas")?;

        let canvas_parent = canvas
            .parent_element()
            .ok_or("Can't find the canvas parent element.")?;

        // disable contextmenu
        let context_menu_callback_ref =
            canvas_add_event_listener(&canvas, "contextmenu", |e: WebEvent| {
                e.prevent_default();
            })?;
        std::mem::forget(context_menu_callback_ref);

        let _ = canvas.focus();
        let dpi = window.device_pixel_ratio();

        let UVec2 {
            x: width,
            y: height,
        } = config.size;
        set_size_dpi(&canvas, width, height);

        Ok(Self {
            canvas,
            win: window,
            document,
            parent: canvas_parent,
            dpi: dpi as f32,
            config,
        })
    }

    fn set_size(&mut self, width: u32, height: u32) {
        set_size_dpi(&self.canvas, width as _, height as _);
        self.config.size = uvec2(width, height);
    }
}

fn get_or_create_canvas(doc: &Document, canvas_id: &str) -> Result<HtmlCanvasElement, String> {
    let canvas = match doc.get_element_by_id(canvas_id) {
        Some(c) => c,
        None => {
            let c = doc.create_element("canvas").map_err(|e| format!("{e:?}"))?;

            let body = doc
                .body()
                .ok_or_else(|| "body doesn't exists on document.".to_string())?;
            body.append_child(&c).map_err(|e| format!("{e:?}"))?;

            c.set_id(canvas_id);
            c
        }
    };

    let canvas_element = canvas
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|e| format!("{e:?}"))?;

    if let Err(e) = canvas_element.style().set_property("touch-action", "none") {
        log::error!("Cannot set touch-action: none {e:?}");
    }

    if let Err(e) = canvas_element.set_attribute("tabindex", "0") {
        log::warn!("Cannot set tabindex to 0, this can lead to errors with focus/unfocus the canvas: {e:?}");
    }

    if let Err(e) = canvas_element.style().set_property("outline", "none") {
        log::error!("Cannot set outline: none {e:?}");
    }

    Ok(canvas_element)
}
