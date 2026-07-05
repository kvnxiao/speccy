# Typing `just` with no args lists every recipe.
default:
    @just --list

# Build the whole workspace.
build:
    cargo build

# Build and install the `speccy` binary from the committed lockfile.
install:
    cargo install --path speccy-cli --locked

# Run unit + integration tests across the workspace.
test:
    cargo test --all

# Apply nightly rustfmt to the whole workspace.
fmt:
    cargo +nightly fmt --all

# CI gate: verify formatting, then clippy with warnings as errors.
lint:
    cargo +nightly fmt --all --check
    cargo clippy --all-targets -- -D warnings

# Accept/reject insta golden snapshot changes.
insta:
    cargo insta review

# Full CI gate run locally: lint (fmt-check + clippy) then tests.
ci: lint test
