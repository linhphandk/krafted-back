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

## Testing
- Always create tests for new features and bug fixes
- Unit tests for service layer (mock repositories via ports)
- Integration tests for repository layer with testcontainers
- Run `nix-shell -p postgresql` before running `cargo test` to provide `libpq` linker

## Local Environment
- Run `nix-shell -p postgresql` before running `cargo test` to provide `libpq` linker

## Refresh Token Strategy (Source: Auth0)
- **Access tokens**: short-lived (minutes/hours), bearer tokens for API access
- **Refresh tokens**: longer-lived, used to obtain new access tokens without re-login
- **Refresh Token Rotation**: every refresh token exchange returns a NEW refresh token; old one is invalidated
- **Automatic Reuse Detection**: if a previously-used refresh token is sent again, invalidate the entire "token family" and require re-authentication
- **Token families**: track all refresh tokens descending from the original; reuse of any member invalidates all
- **Security**: treat all token holders as potentially malicious; rotation + reuse detection mitigates replay attacks
- **Implementation**: store refresh tokens in DB, rotate on every use, detect reuse, invalidate family on conflict
- **Token Delivery**: Return `{ access_token, refresh_token, expires_in }` in JSON response body. No cookies.
