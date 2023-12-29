set windows-shell := ["powershell.exe"]

export RUST_LOG := "info"
export RUST_BACKTRACE := "1"

check:
    cargo check --all --tests
    cargo fmt --all --check

format:
    cargo fmt --all

fix:
    cargo clippy --all --tests --fix

lint:
    cargo clippy --all --tests -- -D warnings

run $app:
    cargo run -r --bin {{app}}

test:
    cargo test --all -- --nocapture

@versions:
    rustc --version
    cargo fmt -- --version
    cargo clippy -- --version

