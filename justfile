_default:
    @just --list

build:
    cargo build

test:
    cargo test

test-integration:
    cargo test -- --include-ignored

lint:
    cargo clippy -- -D warnings

fmt:
    cargo +nightly fmt --all

fmt-check:
    cargo +nightly fmt --all -- --check

deny:
    cargo deny check

ci: fmt-check lint build test deny
    @echo "CI passed."
