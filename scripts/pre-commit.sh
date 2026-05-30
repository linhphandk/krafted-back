#!/bin/bash
set -e

echo "Running cargo fmt..."
cargo fmt --check

echo "Running cargo clippy..."
cargo clippy -- -D warnings

echo "Running tests..."
if command -v nix-shell &> /dev/null; then
    nix-shell -p postgresql --run "cargo test -- --test-threads=1"
else
    cargo test -- --test-threads=1
fi

echo "Pre-commit checks passed."
