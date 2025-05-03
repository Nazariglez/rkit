check-health:
    cargo clippy --all-features
    cargo machete
    cargo deny check
    cargo test --all-features
