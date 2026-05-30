# krafted-back — Auth Template Plan

## Stack
- **Language**: Rust
- **Framework**: Axum
- **ORM**: Diesel
- **Database**: PostgreSQL
- **Identity Provider**: Authentik (OAuth2/OIDC)

## Architecture: Ports & Adapters (Hexagonal), Domain-Scoped

```
src/
  auth/
    mod.rs               # Re-exports
    controller.rs        # Axum handlers, routes, DTOs
    service.rs           # Auth business logic
    ports.rs             # AuthProvider trait
    repository.rs        # AuthentikAuthProvider adapter
    models.rs            # Auth domain models (Token, Session)
  user/
    mod.rs
    controller.rs
    service.rs
    ports.rs             # UserRepository trait
    repository.rs        # DieselUserRepository adapter
    models.rs            # User domain model
  rbac/
    mod.rs
    controller.rs
    service.rs
    ports.rs             # RbacRepository trait
    repository.rs        # DieselRbacRepository adapter
    models.rs            # Role, Permission domain models
  session/
    mod.rs
    service.rs
    ports.rs             # SessionRepository trait
    repository.rs        # DieselSessionRepository adapter
    models.rs            # Session domain model
  shared/
    errors.rs            # Shared error types
    types.rs             # Shared type aliases, newtypes
    config.rs            # App config (Env, Authentik, DB)
    middleware.rs         # Auth guard, RBAC guard
  main.rs
  lib.rs
  router.rs              # Compose all domain routers
migrations/              # Diesel migrations
```

```
┌──────────────────┐
│  Controller.rs   │  Axum handlers, routes, DTOs
└────────┬─────────┘
         │
┌────────▼─────────┐
│    Service.rs     │  Business logic, depends on port traits
└────────┬─────────┘
         │
┌────────▼─────────┐
│     Ports.rs     │  Trait definitions (interfaces)
└────────┬─────────┘
         │ impl
┌────────▼─────────┐
│  Repository.rs   │  Adapters: Diesel queries, Authentik API
└──────────────────┘
```

Each domain module is self-contained. Services depend on port traits; adapters implement them and are injected at runtime.

---

## Milestones

### M1 — Project Scaffolding & Core Architecture
- [ ] Initialize Rust/Axum project with Cargo workspace
- [ ] Set up Diesel with migrations and PostgreSQL config
- [ ] Create domain-scoped folder structure (auth/, user/, rbac/, session/, shared/)
- [ ] Wire up Axum with modular router composition
- [ ] Add CORS, tracing, and env config

### M2 — Database & Repository Layer
- [ ] Create Diesel migrations for: `users`, `roles`, `permissions`, `sessions`, `user_roles`, `role_permissions`
- [ ] Define domain models: `User`, `Role`, `Permission`, `Session`
- [ ] Define port traits per domain:
  - `auth/ports.rs` — `AuthProvider` trait
  - `user/ports.rs` — `UserRepository` trait
  - `session/ports.rs` — `SessionRepository` trait
  - `rbac/ports.rs` — `RbacRepository` trait
- [ ] Implement adapters per domain:
  - `auth/repository.rs` — `AuthentikAuthProvider`
  - `user/repository.rs` — `DieselUserRepository`
  - `session/repository.rs` — `DieselSessionRepository`
  - `rbac/repository.rs` — `DieselRbacRepository`
- [ ] Add integration tests for repositories

### M3 — Authentik Integration (External Adapter)
- [ ] Deploy/configure Authentik (Docker Compose for dev)
- [ ] Define port trait: `AuthProvider` — `register()`, `login()`, `introspect_token()`, `refresh_token()`, `revoke_token()`
- [ ] Implement `AuthentikAuthProvider` adapter (`src/auth/repository.rs`):
  - **`register()`**: POST to Authentik admin API to create user with email, name, password; return `UserInfo` (sub, email, name)
  - **`login()`**: POST to Authentik `/token` with `grant_type=password` (ROPC flow); parse response (`access_token`, `refresh_token`, `id_token`); validate `id_token` JWT signature via JWKS; extract user claims (sub, email, name)
  - **`introspect_token()`**: verify JWT locally — decode, verify signature against JWKS public key, check `exp`, `iss`, `aud` claims; fallback to POST `/introspect` if needed
  - **`refresh_token()`**: POST to `/token` with `grant_type=refresh_token`; return new `access_token` (and optionally new `refresh_token`)
  - **`revoke_token()`**: POST to `/revoke` endpoint; invalidate token server-side
  - **JWKS cache**: fetch `/.well-known/jwks.json`, cache public keys, handle key rotation (refetch on signature mismatch)
  - **OIDC Discovery**: fetch `/.well-known/openid-configuration` to auto-discover token, introspect, revoke, JWKS endpoints
- [ ] Wire `AuthentikAuthProvider` into `AppState` and inject into `AuthService`

### M4 — Service Layer (Business Logic)
- [ ] Implement `AuthService`:
  - `register()` — provision user in Authentik + local DB (done)
  - `login()` — delegate to AuthProvider, create local session
  - `logout()` — revoke session
  - `refresh_token()` — handle token refresh
- [ ] Implement `UserService`:
  - `get_user()`, `update_user()`, `delete_user()`
- [ ] Implement `RBACService`:
  - `assign_role()`, `revoke_role()`
  - `check_permission()`, `has_role()`
- [ ] All services depend on port interfaces, never concrete adapters

### M5 — Controller Layer (HTTP API)
- [ ] Auth endpoints:
  - `POST /api/auth/register` — register user (done)
  - `POST /api/auth/login` — authenticate with credentials, return tokens
  - `POST /api/auth/logout` — revoke session
  - `POST /api/auth/refresh` — refresh tokens
  - `GET  /api/auth/me` — current user info
- [ ] User management:
  - `GET    /api/users` — list users (admin)
  - `GET    /api/users/:id` — get user
  - `PATCH  /api/users/:id` — update user
  - `DELETE /api/users/:id` — delete user
- [ ] RBAC endpoints:
  - `GET    /api/roles` — list roles
  - `POST   /api/roles` — create role (admin)
  - `POST   /api/users/:id/roles` — assign role
  - `DELETE /api/users/:id/roles/:role` — revoke role
- [ ] Middleware: auth guard (extract+validate JWT), RBAC guard (check permissions)
- [ ] Error handling: map domain errors to proper HTTP responses

### M7 — Authorization Logic (RBAC + ABAC)
- [ ] Define seed data: default roles (`admin`, `user`, `editor`) and permissions
- [ ] Implement permission checks in service layer
- [ ] Implement ABAC policies as needed (resource-level checks)
- [ ] Middleware to extract roles/permissions from verified JWT claims
- [ ] Admin-only endpoints enforced

### M8 — Testing & Documentation
- [ ] Unit tests: service layer (mock repositories via ports)
- [ ] Integration tests: repository layer with test DB
- [ ] API docs (OpenAPI/Swagger via utoipa)
- [ ] Architecture decision records for key choices
- [ ] Runnable Docker Compose (Postgres + Authentik + backend)

---

## Key Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Auth flow | ROPC (password grant) | Users login via app UI, credentials verified by Authentik |
| Credentials | Stored in Authentik | No password hashing in local DB, SSO support |
| Profile data | Stored in local DB | App-specific data (name, roles, preferences) |
| Token format | JWT (issued by Authentik) | Stateless verification via JWKS |
| Session strategy | Local session table + JWT | Revocation support via DB |
| Repository pattern | Diesel with traits | Type-safe, testable |
| Config | dotenv + envy | Typed, validated config |

## Port Interfaces (Trait Definitions)

```rust
// src/auth/ports.rs
pub trait AuthProvider: Send + Sync {
    async fn register(&self, email: &str, name: &str, password: &str) -> AppResult<UserInfo>;
    async fn login(&self, email: &str, password: &str) -> AppResult<(Tokens, UserInfo)>;
    async fn introspect_token(&self, token: &str) -> AppResult<UserInfo>;
    async fn refresh_token(&self, refresh_token: &str) -> AppResult<Tokens>;
    async fn revoke_token(&self, token: &str) -> AppResult<()>;
}

// src/user/ports.rs
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn create(&self, user: NewUser) -> Result<User>;
    async fn update(&self, id: Uuid, data: UpdateUser) -> Result<User>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}

// src/session/ports.rs
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: NewSession) -> Result<Session>;
    async fn find_by_token(&self, token: &str) -> Result<Option<Session>>;
    async fn revoke(&self, token: &str) -> Result<()>;
}

// src/rbac/ports.rs
pub trait RbacRepository: Send + Sync {
    async fn assign_role(&self, user_id: Uuid, role_id: Uuid) -> Result<()>;
    async fn revoke_role(&self, user_id: Uuid, role_id: Uuid) -> Result<()>;
    async fn get_permissions(&self, user_id: Uuid) -> Result<Vec<Permission>>;
    async fn has_permission(&self, user_id: Uuid, permission: &str) -> Result<bool>;
}
```

## Adapters (Implementations)

| Port | Adapter | Location | Technology |
|---|---|---|---|
| `AuthProvider` | `AuthentikAuthProvider` | `src/auth/repository.rs` | Authentik API (ROPC + admin) |
| `UserRepository` | `DieselUserRepository` | `src/user/repository.rs` | Diesel + PostgreSQL |
| `SessionRepository` | `DieselSessionRepository` | `src/session/repository.rs` | Diesel + PostgreSQL |
| `RbacRepository` | `DieselRbacRepository` | `src/rbac/repository.rs` | Diesel + PostgreSQL |

## Dependency Graph

```
src/auth/controller.rs
         ↓
src/auth/service.rs
         ↓  (depends on traits)
         ├─ src/auth/ports.rs  ←  src/auth/repository.rs (adapter impl)
         └─ src/user/ports.rs  ←  src/user/repository.rs (adapter impl)

src/user/controller.rs
         ↓
src/user/service.rs
         ↓  (depends on traits)
src/user/ports.rs  ←  src/user/repository.rs (adapter impl)

(same pattern for session/, rbac/)
```