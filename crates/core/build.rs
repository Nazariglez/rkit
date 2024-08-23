fn main() {
    // Check if we're building tests
    if cfg!(feature = "test-env") {
        // Set environment variables for the test build
        println!("cargo:rustc-env=EXISTING_ENV_VAR=123"); // used in utils.rs
        println!("cargo:rustc-env=OVERFLOW_ENV_VAR=999999999999999999999999"); // used in utils.rs
    }
}
