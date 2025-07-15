use std::{fmt::Display, panic::PanicHookInfo};

pub trait PanicContext<T, E> {
    fn or_panic(self, ctx: &str) -> T;
    fn or_panic_with<F, S>(self, f: F) -> T
    where
        S: Display,
        F: FnOnce() -> S;
}

impl<T, E> PanicContext<T, E> for Result<T, E>
where
    E: Display,
{
    #[track_caller]
    #[cold]
    #[inline(never)]
    fn or_panic(self, ctx: &str) -> T {
        match self {
            Ok(t) => t,
            Err(err) => {
                log::error!("{ctx}: {err}");
                panic!("{ctx}: {err}")
            }
        }
    }

    #[track_caller]
    #[cold]
    #[inline(never)]
    fn or_panic_with<F, S>(self, f: F) -> T
    where
        S: Display,
        F: FnOnce() -> S,
    {
        match self {
            Ok(t) => t,
            Err(err) => {
                let ctx = f();
                log::error!("{ctx}: {err}");
                panic!("{ctx}: {err}")
            }
        }
    }
}

impl<T> PanicContext<T, ()> for Option<T> {
    #[track_caller]
    #[cold]
    #[inline(never)]
    fn or_panic(self, ctx: &str) -> T {
        match self {
            Some(t) => t,
            None => {
                log::error!("{ctx}");
                panic!("{ctx}")
            }
        }
    }

    #[track_caller]
    #[cold]
    #[inline(never)]
    fn or_panic_with<F, S>(self, f: F) -> T
    where
        S: Display,
        F: FnOnce() -> S,
    {
        match self {
            Some(t) => t,
            None => {
                let ctx = f();
                log::error!("{ctx}");
                panic!("{ctx}")
            }
        }
    }
}

#[allow(dead_code)]
fn panic_hook(info: &PanicHookInfo) {
    let msg = info
        .payload()
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());

    let loc = info
        .location()
        .map(|l| l.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    println!("----------------");
    println!("> CRASH REPORT <\nReason: '{msg}'\nLocation: '{loc}'");
    println!("----------------");
}

pub fn enable_crash_report_on_panic() {
    #[cfg(all(not(target_arch = "wasm32"), not(debug_assertions)))]
    std::panic::set_hook(Box::new(panic_hook));
}
