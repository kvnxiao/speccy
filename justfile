run args:
    cargo run --quiet -p speccy-cli -- {{args}}

build:
    cargo build --workspace --release

reeject: install
    speccy init --force --host claude-code && speccy init --force --host codex

install:
    cargo install --path speccy-cli

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo +nightly fmt --all --check

test:
    cargo test --workspace
