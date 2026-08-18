use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        windowed: { any(target_arch = "wasm32", not(feature = "headless")) },
        native_windowed: { all(not(target_arch = "wasm32"), not(feature = "headless")) },
        native_headless: { all(not(target_arch = "wasm32"), feature = "headless") },
    }

    if cfg!(feature = "test-env") {
        println!("cargo:rustc-env=EXISTING_ENV_VAR=123");
        println!("cargo:rustc-env=OVERFLOW_ENV_VAR=999999999999999999999999");
    }
}
