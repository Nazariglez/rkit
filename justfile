check-health:
    cargo clippy --all-features
    cargo machete
    cargo deny check
    SAVE_FILE_DIR=./fixtures cargo test --workspace --all-features
