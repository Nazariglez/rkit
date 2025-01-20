check-health:
    cargo clippy --all-features
    cargo machete
    cargo audit
    cargo test --all-features
