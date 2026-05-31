# M8 — Testing & Documentation

**Deliverable: Unit tests for all services, integration tests for all repositories and E2E flows, OpenAPI/Swagger docs complete, Docker Compose runs the full stack**

**Depends on: M7.1**

---

## Principle: Tests Prove Behavior, Docs Prove API Contract

Every service has unit tests with mock repositories. Every repository has integration tests against a real PostgreSQL (testcontainers). Every auth flow has E2E tests hitting the full HTTP stack. OpenAPI docs are auto-generated from `utoipa` annotations and accessible via Scalar UI.

---

## Task 1 — Unit tests for all services (expand existing) (~200loc)

**Files**: `tests/auth_service.rs`, `tests/user_service.rs`, `tests/rbac_service.rs`

Service unit tests use `mockall` to mock repositories. Each test follows the pattern: set up mocks → call service method → assert result.

Expand existing test suites:

`tests/auth_service.rs` — already has comprehensive tests. Verify:
- `test_register_success`
- `test_register_empty_email`
- `test_register_short_password`
- `test_login_success`
- `test_login_user_not_found`
- `test_login_wrong_password`
- `test_logout_success`
- `test_refresh_token_success`
- `test_refresh_token_expired`
- `test_refresh_token_invalid`
- `test_get_current_user_success`

`tests/user_service.rs` — verify:
- `test_create_user`
- `test_find_by_email_found`
- `test_find_by_email_not_found`
- `test_find_by_id_found`
- `test_find_by_id_not_found`
- `test_list_users`
- `test_count_users`
- `test_update_user`
- `test_soft_delete_user`

`tests/rbac_service.rs` — verify:
- `test_assign_default_role`
- `test_assign_admin_role`
- `test_assign_default_role_no_role_found`
- `test_get_user_permissions`
- `test_get_user_permissions_no_roles`

**Estimated lines**: ~200loc (new test cases for user service, expanded RBAC tests)

---

## Task 2 — Integration tests for all repositories (expand existing) (~150loc)

**Files**: `tests/user_repository.rs`, `tests/session_repository.rs`, `tests/rbac_repository.rs`

Each test uses `testcontainers` to spin up PostgreSQL and tests real database operations.

Expand existing tests:

`tests/user_repository.rs` — add:
- `test_update_user` — update name, verify only name changed
- `test_soft_delete_user` — set `is_active = false`, verify user not in list results
- `test_list_users_paginated` — create multiple users, verify pagination
- `test_count_users` — verify count matches created users
- `test_find_by_email_unique_constraint` — duplicate email returns error

`tests/session_repository.rs` — already has:
- `test_create_session`
- `test_find_by_token`
- `test_revoke_session`
- Add: `test_revoke_nonexistent_session` — no error on revoking missing token

`tests/rbac_repository.rs` — add:
- `test_find_role_by_id` — find admin role by UUID
- `test_revoke_role` — assign then revoke, verify
- `test_list_roles` — verify both seeded roles returned
- `test_get_permission_names_empty` — empty role IDs returns empty vec

**Estimated lines**: ~150loc (new test cases)

---

## Task 3 — E2E test coverage review and expansion (~200loc)

**Files**: `tests/auth_e2e.rs`, `tests/user_e2e.rs` (new), `tests/rbac_e2e.rs` (new)

`tests/auth_e2e.rs` — verify all flows:
- Register → login → me → refresh → logout
- Error cases: duplicate email, wrong password, expired token, missing token
- RBAC: register assigns `user` role, JWT contains role

`tests/user_e2e.rs` — verify user management:
- List users (paginated)
- Get user by ID
- Update user name
- Update user email
- Soft delete user
- Auth required on all endpoints

`tests/rbac_e2e.rs` — verify RBAC endpoints:
- List roles (any authenticated user)
- Assign role (admin only)
- Revoke role (admin only)
- Non-admin gets 403

**Estimated lines**: ~200loc (new E2E tests)

---

## Task 4 — Error handling consistency (~30loc)

**File**: `src/shared/errors.rs` (expand)

Add missing error variants that are needed by M6/M7.1:

```rust
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Internal error")]
    Internal,
    #[error("Not implemented")]
    NotImplemented,
}
```

Update `IntoResponse` to map each variant to the correct HTTP status:
- `BadRequest` → 400
- `NotFound` → 404
- `Unauthorized` → 401
- `Forbidden` → 403
- `Internal` → 500
- `NotImplemented` → 501

**Estimated lines**: ~30loc

---

## Task 5 — Pre-commit hooks and CI-ready test commands (~15loc)

**File**: `scripts/pre-commit.sh` (update)

```bash
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
```

Ensure this is git hook-ready. Add to `.husky/pre-commit` or equivalent.

**Estimated lines**: ~15loc

---

## Task 6 — Complete OpenAPI documentation (~30loc)

**File**: `src/api_doc.rs`

Ensure all endpoints from M5, M6, M7.1 are documented:

```rust
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::auth::controller::register,
        crate::auth::controller::login,
        crate::auth::controller::logout,
        crate::auth::controller::refresh,
        crate::auth::controller::me,
        crate::user::controller::list_users,
        crate::user::controller::get_user,
        crate::user::controller::update_user,
        crate::user::controller::delete_user,
        crate::rbac::controller::list_roles,
        crate::rbac::controller::assign_role,
        crate::rbac::controller::revoke_role,
    ),
    components(schemas(
        RegisterRequest, LoginRequest, LogoutRequest, RefreshRequest,
        RegisterResponse, LoginResponse, RefreshResponse, UserResponse,
        UpdateUserRequest, PaginatedResponse<UserResponse>,
        RoleResponse, AssignRoleRequest, RevokeRoleRequest,
        ErrorResponse,
    )),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "rbac", description = "Role and permission management endpoints"),
    ),
)]
pub struct ApiDoc;
```

**Estimated lines**: ~30loc

---

## Task 7 — Docker Compose for full stack (~20loc)

**File**: `docker-compose.yml` (update)

Add a backend service (for production-like testing):

```yaml
services:
  db:
    container_name: db
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_USER: krafted
      POSTGRES_PASSWORD: krafted
      POSTGRES_DB: krafted
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U krafted -d krafted"]
      interval: 5s
      timeout: 5s
      retries: 5

  backend:
    build: .
    ports:
      - "3000:3000"
    environment:
      SERVER_HOST: 0.0.0.0
      SERVER_PORT: 3000
      DATABASE_URL: postgres://krafted:krafted@db:5432/krafted
      DATABASE_POOL_SIZE: 4
      JWT_SECRET: change-me-to-a-random-string
      JWT_EXPIRY_MINUTES: 15
      ADMIN_EMAIL: admin@example.com
      ADMIN_PASSWORD: adminpassword
    depends_on:
      db:
        condition: service_healthy

  adminer:
    image: adminer:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
    depends_on:
      db:
        condition: service_healthy

volumes:
  pgdata:
```

Also add a `Dockerfile` for the Rust backend:

```dockerfile
FROM rust:1.82 AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY diesel.toml .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libpq5 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/krafted-back /usr/local/bin/
EXPOSE 3000
CMD ["krafted-back"]
```

**Estimated lines**: ~50loc (docker-compose + Dockerfile)

---

## Task 8 — Architecture decision records (~100loc)

**File**: `docs/adr/` (new directory)

Create ADRs for key decisions:
1. `001-local-auth-vs-oauth2.md` — Why we chose local auth (bcrypt + JWT) over Authentik/OAuth2
2. `002-hexagonal-architecture.md` — Why ports & adapters for testability
3. `003-diesel-over-sqlx.md` — Why Diesel for type-safe queries
4. `004-refresh-token-rotation.md` — Why we rotate refresh tokens (Auth0 strategy)
5. `005-rbac-in-jwt.md` — Why we embed role/permissions in JWT claims vs. DB lookup per request

Each ADR follows the format: Context → Decision → Consequences.

**Estimated lines**: ~100loc (5 ADRs × ~20loc each)

---

## Verification

After completing all tasks, you should be able to:

1. `cargo test -- --test-threads=1` — all unit, integration, and E2E tests pass
2. `cargo fmt --check` — no formatting issues
3. `cargo clippy -- -D warnings` — no warnings
4. `docker compose up` — PostgreSQL and backend start, admin user seeded
5. `GET /scalar` — complete API documentation with all endpoints
6. `GET /api-docs/openapi.json` — valid OpenAPI 3.0 spec
7. All error responses use `{ "message": "..." }` format with correct HTTP status codes
8. Pre-commit hook runs format, clippy, and tests
9. Architecture decision records exist for the 5 key decisions
10. `cargo run` with `ADMIN_EMAIL` and `ADMIN_PASSWORD` env vars seeds the admin user