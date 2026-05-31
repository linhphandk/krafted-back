# M1 — Project Scaffolding & Core Architecture

**Deliverable: Rust/Axum project boots, health check responds, domain-scoped folder structure in place, Diesel migrations run, CORS and tracing configured**

**Depends on: Nothing (first milestone)**

---

## Principle: Ports & Adapters, Domain-Scoped

Each domain module (`auth/`, `user/`, `rbac/`, `session/`) is self-contained. Within a module, code flows top-to-bottom: `models → ports → service → repository → controller`. Services depend on port traits (interfaces), never concrete adapters. Adapters implement those traits and are injected at runtime via `AppState`.

```
┌──────────────────┐
│  Controller.rs   │  Axum handlers, routes, DTOs
└────────┬─────────┘
         │
┌────────▼─────────┐
│    Service.rs      │  Business logic, depends on port traits
└────────┬─────────┘
         │
┌────────▼─────────┐
│     Ports.rs      │  Trait definitions (interfaces)
└────────┬─────────┘
         │ impl
┌────────▼─────────┐
│  Repository.rs    │  Adapters: Diesel queries, Local Auth logic
└──────────────────┘
```

---

## Task 1 — Initialize Rust/Axum project (~30loc config)

**Files**: `Cargo.toml`, `.env.example`, `diesel.toml`

`Cargo.toml` dependencies:
```toml
[dependencies]
axum = "0.8"
http = "1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
diesel = { version = "2", features = ["postgres", "r2d2", "uuid", "chrono"] }
diesel_migrations = "2"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
dotenvy = "0.15"
envy = "0.4"
thiserror = "1"
reqwest = { version = "0.12", features = ["json"] }
bcrypt = "0.17"
jsonwebtoken = "9"
async-trait = "0.1"
mockall = "0.12"
utoipa = { version = "5", features = ["axum_extras"] }
utoipa-scalar = { version = "0.3", features = ["axum"] }

[dev-dependencies]
tokio-test = "0.4"
testcontainers = "0.15"
testcontainers-modules = { version = "0.3", features = ["postgres"] }
tower = "0.5"
```

`.env.example`:
```
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
DATABASE_URL=postgres://krafted:kreated@localhost:5432/krafted
DATABASE_POOL_SIZE=4
JWT_SECRET=change-me-to-a-random-string
JWT_EXPIRY_MINUTES=15
RUST_LOG=info
```

`diesel.toml`:
```toml
[print_schema]
file = "src/schema.rs"
```

**Estimated lines**: ~60loc (Cargo.toml + configs)

---

## Task 2 — Domain-scoped folder structure (~40loc)

**Files**: All module directories with `mod.rs` barrel exports

Create the full domain-scoped directory structure:

```
src/
  auth/
    mod.rs               # Re-exports + auth_router()
    controller.rs        # Axum handlers, routes, DTOs
    service.rs           # Auth business logic
    ports.rs             # AuthProvider trait
    provider.rs          # LocalAuthProvider adapter (was repository.rs in PLAN.md)
    models.rs            # Auth domain models (Tokens, UserInfo)
  user/
    mod.rs
    models.rs            # User, NewUser
    ports.rs             # UserRepository trait
    repository.rs        # DieselUserRepository adapter
    service.rs           # UserService
  rbac/
    mod.rs
    models.rs            # Role, Permission, UserRole, RolePermission
    ports.rs             # RbacRepository trait
    repository.rs        # DieselRbacRepository adapter
    service.rs           # RbacService
  session/
    mod.rs
    models.rs            # Session, NewSession
    ports.rs             # SessionRepository trait
    repository.rs        # DieselSessionRepository adapter
  shared/
    mod.rs
    errors.rs            # AppError, AppResult, ErrorResponse
    types.rs             # DbResult, AuthenticatedUser
    config.rs            # Config struct (env-based)
    db.rs                 # establish_pool, run_migrations
    middleware.rs         # auth_middleware (JWT guard)
  main.rs                # Server entry point
  lib.rs                 # Module declarations
  router.rs              # Compose all domain routers
  state.rs               # AppState (wiring)
  schema.rs              # Diesel-generated schema
  api_doc.rs             # utoipa OpenAPI spec
migrations/               # Diesel migrations
```

Each `mod.rs` re-exports public API. `auth/mod.rs` also defines `auth_router()` which mounts public and protected routes.

**Estimated lines**: ~40loc (mod.rs files)

---

## Task 3 — Config and database setup (~60loc)

**Files**: `src/shared/config.rs`, `src/shared/db.rs`

`src/shared/config.rs`:
- `Config` struct with fields: `server_host`, `server_port`, `database_url`, `database_pool_size`, `jwt_secret`, `jwt_expiry_minutes`
- `Config::from_env()` using `envy::from_env()`
- All fields validated at startup

`src/shared/db.rs`:
- `DbPool` type alias for `r2d2::Pool<ConnectionManager<PgConnection>>`
- `establish_pool()` — creates an r2d2 connection pool
- `run_migrations()` — runs embedded Diesel migrations

**Estimated lines**: ~40loc

---

## Task 4 — Error types and shared types (~60loc)

**Files**: `src/shared/errors.rs`, `src/shared/types.rs`

`src/shared/errors.rs`:
```rust
pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal error")]
    Internal,
    #[error("Not implemented")]
    NotImplemented,
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}

impl IntoResponse for AppError { /* maps to HTTP status codes */ }
```

`src/shared/types.rs`:
```rust
pub type DbResult<T> = Result<T, diesel::result::Error>;

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
}
```

**Estimated lines**: ~60loc

---

## Task 5 — Wire up Axum with modular router composition (~50loc)

**Files**: `src/router.rs`, `src/state.rs`, `src/main.rs`

`src/router.rs`:
- `create_router(state: AppState) -> Router` — health check, OpenAPI JSON, Scalar UI, and auth routes
- Public routes: `/health`, `/api-docs/openapi.json`, `/scalar`
- Domain routers merged in

`src/state.rs`:
- `AppState` struct holding `auth_service`, with `new()` constructor
- Dependency injection: `LocalAuthProvider`, `DieselUserRepository`, `DieselSessionRepository`, `DieselRbacRepository` all wired together
- `RbacService` wrapped in `Arc` for shared state

`src/main.rs`:
- Load `.env`, configure tracing
- Parse `Config::from_env()`
- Establish pool, run migrations
- Build `AppState`
- Create router with CORS layer (allow all origins for dev)
- Bind to `server_host:server_port` and serve

**Estimated lines**: ~100loc combined

---

## Task 6 — Docker Compose for local PostgreSQL (~30loc)

**Files**: `docker-compose.yml`

PostgreSQL 16 + Adminer for local development:
```yaml
services:
  db:
    image: postgres:16-alpine
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

  adminer:
    image: adminer:latest
    ports:
      - "8080:8080"
    depends_on:
      db:
        condition: service_healthy

volumes:
  pgdata:
```

**Estimated lines**: ~30loc

---

## Task 7 — OpenAPI documentation scaffolding (~40loc)

**Files**: `src/api_doc.rs`

Use `utoipa` to generate OpenAPI spec. Initially only includes auth endpoints:

```rust
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::auth::controller::register,
        crate::auth::controller::login,
        crate::auth::controller::logout,
        crate::auth::controller::refresh,
        crate::auth::controller::me,
    ),
    components(schemas(/* DTOs */)),
    tags((name = "auth", description = "Authentication endpoints")),
)]
pub struct ApiDoc;
```

Mounted in `router.rs` at `/api-docs/openapi.json` and `/scalar`.

**Estimated lines**: ~40loc

---

## Verification

After completing all tasks, you should be able to:

1. `cargo check` — compiles without errors
2. `cargo fmt --check` — passes formatting
3. `cargo clippy -- -D warnings` — no warnings
4. `docker compose up -d` — PostgreSQL running on port 5432
5. `cargo run` — server starts on configured host:port
6. `GET /health` — returns `"OK"`
7. `GET /api-docs/openapi.json` — returns OpenAPI JSON
8. `GET /scalar` — renders Scalar API documentation UI
9. Domain modules are correctly structured: each has `mod.rs`, `models.rs`, `ports.rs` (where applicable), and `service.rs`/`repository.rs`/`controller.rs` (where applicable)
10. `AppState` wires all dependencies together with trait-based injection