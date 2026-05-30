#!/bin/bash
set -e

echo "Running cargo fmt..."
cargo fmt --check

echo "Running cargo clippy..."
cargo clippy -- -D warnings

echo "Running tests..."
if command -v nix-shell &> /dev/null; then
    nix-shell -p postgresql --run "cargo test"
else
    cargo test
fi

echo "Pre-commit checks passed."
