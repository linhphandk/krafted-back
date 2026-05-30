# Development Guidelines

## Workflow
- Implement features **vertically** (end-to-end) within a single domain/module
- Split by layer: migrations+repository → service → controller
- Each PR should be small, focused, and independently reviewable
- **Always stop after completing a task** for PR review — do not continue to the next task without approval

## Architecture
- Ports & Adapters (Hexagonal), Domain-Scoped
- Each domain module is self-contained: model → port → service → repository → controller
- Services depend on port traits, never concrete adapters
- Adapters implement port traits and are injected at runtime

## Code Style
- No comments unless explicitly requested
- Follow existing conventions in the codebase
- Run `cargo fmt`, `cargo clippy`, `cargo check` before committing

## Local Environment
- Run `nix-shell -p postgresql` before running `cargo test` to provide `libpq` linker
