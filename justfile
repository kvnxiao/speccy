run args:
    cargo run --quiet -p speccy-cli -- {{args}}

build:
    cargo build --workspace --release

install:
    cargo install --path speccy-cli

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo +nightly fmt --all --check

test:
    cargo test --workspace
