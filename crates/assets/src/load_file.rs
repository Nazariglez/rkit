#[cfg(not(target_arch = "wasm32"))]
use futures::channel::oneshot;
#[cfg(not(target_arch = "wasm32"))]
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::future::Future;

#[cfg(target_arch = "wasm32")]
use futures_util::future::{TryFutureExt, poll_fn, ready};
#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use std::task::{Context, Poll};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
#[cfg(target_arch = "wasm32")]
use web_sys::{XmlHttpRequest, XmlHttpRequestResponseType};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct FileLoader {
    thread_pool: ThreadPool,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileLoader {
    pub fn new() -> Result<Self, String> {
        let thread_pool = ThreadPoolBuilder::default()
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { thread_pool })
    }

    pub fn load_file(&self, path: &str) -> impl Future<Output = Result<Vec<u8>, String>> + use<> {
        let (tx, rx) = oneshot::channel();

        let path = path.to_owned();
        self.thread_pool.spawn(move || {
            let read_result = std::fs::read(&path);
            let _ = tx.send(read_result.map_err(|e| e.to_string()));
        });

        async move {
            rx.await
                .unwrap_or_else(|_| Err("The channel was dropped.".to_string()))
        }
    }
}

// The web logic to make the request is based on the crate 'platter' from Ryan Goldstein

#[cfg(target_arch = "wasm32")]
pub(crate) struct FileLoader {}

#[cfg(target_arch = "wasm32")]
impl FileLoader {
    pub fn new() -> Result<Self, String> {
        Ok(Self {})
    }

    pub fn load_file(&self, path: &str) -> impl Future<Output = Result<Vec<u8>, String>> {
        ready(create_request(path)).and_then(|xhr| {
            let mut have_set_handlers = false;
            poll_fn(move |ctx| poll_request(&xhr, ctx, &mut have_set_handlers))
        })
    }
}

#[cfg(target_arch = "wasm32")]
fn err_format(err: JsValue) -> String {
    format!("{:?}", err)
}

// Nasty trick to wrap XmlHttpRequest and make it Send so we bypass
// the compiler errors about not being safe sharing *mut u8 (internal ref on this object)
// this is sound for wasm32 because is single thread
#[cfg(target_arch = "wasm32")]
struct Xhr(XmlHttpRequest);
#[cfg(target_arch = "wasm32")]
unsafe impl Send for Xhr {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for Xhr {}

#[cfg(target_arch = "wasm32")]
fn create_request(path: &str) -> Result<Xhr, String> {
    let xhr = XmlHttpRequest::new().map_err(err_format)?;
    xhr.open("GET", path).map_err(err_format)?;
    xhr.set_response_type(XmlHttpRequestResponseType::Arraybuffer);
    xhr.send().map_err(err_format)?;
    Ok(Xhr(xhr))
}

#[cfg(target_arch = "wasm32")]
fn poll_request(
    xhr: &Xhr,
    ctx: &mut Context,
    have_set_handlers: &mut bool,
) -> Poll<Result<Vec<u8>, String>> {
    if !*have_set_handlers {
        *have_set_handlers = true;
        let waker = ctx.waker().clone();
        let wake_up = Closure::wrap(Box::new(move || waker.wake_by_ref()) as Box<dyn FnMut()>);
        let wake_up_ref = wake_up.as_ref().unchecked_ref();
        xhr.0.set_onload(Some(wake_up_ref));
        xhr.0.set_onerror(Some(wake_up_ref));
        wake_up.forget();
    }
    let status = xhr
        .0
        .status()
        .expect("Failed to get the XmlHttpRequest status");
    let ready_state = xhr.0.ready_state();
    match (status / 100, ready_state) {
        (2, 4) => Poll::Ready(
            xhr.0
                .response()
                .map(|resp| {
                    let array = Uint8Array::new(&resp);
                    let mut buffer = vec![0; array.length() as usize];
                    array.copy_to(&mut buffer[..]);

                    buffer
                })
                .map_err(err_format),
        ),
        (2, _) => Poll::Pending,
        (0, _) => Poll::Pending,
        _ => Poll::Ready(Err("Non-200 status code returned".to_string())),
    }
}
