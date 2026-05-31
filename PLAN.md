# krafted-back — Auth Template Plan

## Stack
- **Language**: Rust
- **Framework**: Axum
- **ORM**: Diesel
- **Database**: PostgreSQL
- **Identity Provider**: Local Auth (bcrypt + JWT)

## Architecture: Ports & Adapters (Hexagonal), Domain-Scoped

```
src/
  auth/
    mod.rs               # Re-exports
    controller.rs        # Axum handlers, routes, DTOs
    service.rs           # Auth business logic
    ports.rs             # AuthProvider trait
    repository.rs        # LocalAuthProvider adapter
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
    config.rs            # App config (Env, JWT, DB)
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
│  Repository.rs   │  Adapters: Diesel queries, Local Auth logic
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
  - `auth/repository.rs` — `LocalAuthProvider`
  - `user/repository.rs` — `DieselUserRepository`
  - `session/repository.rs` — `DieselSessionRepository`
  - `rbac/repository.rs` — `DieselRbacRepository`
- [ ] Add integration tests for repositories

### M3 — Local Auth Implementation
- [ ] Define port trait: `AuthProvider` — `register()`, `login()`, `introspect_token()`, `refresh_token()`, `revoke_token()`
- [ ] Implement `LocalAuthProvider` adapter (`src/auth/repository.rs`):
  - **`register()`**: Hash password with `bcrypt`, call `UserRepository.create()`, return `UserInfo`
  - **`login()`**: Find user by email, verify password with `bcrypt`, generate JWT access token + refresh token (UUID), save refresh token to `sessions` table, return `(Tokens, UserInfo)`
  - **`introspect_token()`**: Decode and verify JWT signature using local secret, check `exp`, return `UserInfo`
  - **`refresh_token()`**: Find refresh token in `sessions` table, verify not expired, rotate token, issue new access token
  - **`revoke_token()`**: Delete refresh token from `sessions` table
- [ ] Add JWT config (`JWT_SECRET`, `JWT_EXPIRY_MINUTES`)
- [ ] Update `AppState` to inject `LocalAuthProvider`

### M4 — Service Layer (Business Logic)
- [ ] Implement `AuthService`:
  - `register()` — hash password in DB (done)
  - `login()` — delegate to `AuthProvider`, create session
  - `logout()` — revoke session
  - `refresh_token()` — handle token rotation
- [ ] Implement `UserService`:
  - `get_user()`, `update_user()`, `delete_user()`
- [ ] Implement `RBACService`:
  - `assign_role()`, `revoke_role()`
  - `check_permission()`, `has_role()`
- [ ] All services depend on port interfaces, never concrete adapters

### M5 — Controller Layer (HTTP API)
- [ ] Auth endpoints:
  - `POST /api/auth/register` — register user (done)
  - `POST /api/auth/login` — authenticate, return JWT + refresh cookie
  - `POST /api/auth/logout` — revoke refresh token
  - `POST /api/auth/refresh` — rotate refresh token, return new JWT
  - `GET  /api/auth/me` — current user info (protected)
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
- [ ] Middleware: auth guard (verify JWT), RBAC guard (check permissions)
- [ ] Error handling: map domain errors to proper HTTP responses

### M7 — Authorization Logic (RBAC)
- [ ] Create migration: `roles`, `permissions`, `user_roles`, `role_permissions` tables with FK constraints
- [ ] Seed data: `user` role, `admin` role, `users:read` + `users:write` permissions
- [ ] Seed `role_permissions`: `user` role gets `users:read` + `users:write`; `admin` role gets all
- [ ] RBAC models: `Role`, `Permission`, `UserRole`, `RolePermission`
- [ ] `RbacRepository` trait: `find_role_by_name()`, `assign_role()`, `get_user_role_ids()`, `get_permission_names_by_role_ids()`
- [ ] `DieselRbacRepository` adapter (2-query strategy, no JOINs — subquery for permissions)
- [ ] `RbacService`: `assign_default_role()`, `get_user_permissions()`
- [ ] On register: assign `user` role via `RbacService.assign_default_role()`
- [ ] On login/refresh: load role + permissions via `get_user_permissions()`, embed in JWT claims
- [ ] Update `AuthProvider.generate_access_token()` to accept role + permissions
- [ ] Update `LocalAuthProvider` to include `role` and `permissions` in JWT claims
- [ ] Update `AuthenticatedUser` to include `role: String` and `permissions: Vec<String>`
- [ ] Update auth middleware to extract role + permissions from JWT
- [ ] Tests: register → role assigned, login → JWT has correct claims
- [ ] No RBAC endpoints yet (no GET/POST for roles or permissions)
- [ ] No admin seed user yet

### M7.1 — Admin Seed & RBAC Endpoints (Future)
- [ ] Seed admin user on startup (env-var configured email/password)
- [ ] `POST /api/users/:id/roles` — assign role (admin-only)
- [ ] `DELETE /api/users/:id/roles/:role` — revoke role (admin-only)
- [ ] Admin-only middleware guard
- [ ] Permission enforcement in handlers

### M8 — Testing & Documentation
- [ ] Unit tests: service layer (mock repositories via ports)
- [ ] Integration tests: repository layer with test DB
- [ ] API docs (OpenAPI/Swagger via utoipa)
- [ ] Architecture decision records for key choices
- [ ] Runnable Docker Compose (Postgres + backend)

---

## Key Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Auth flow | Local Auth + JWT | Simplest, no external IdP, full control |
| Credentials | Stored in local DB (bcrypt) | Zero external dependencies, secure hashing |
| Token format | Self-signed JWT | Stateless verification, no JWKS needed |
| Session strategy | Refresh token in DB | Revocation support, rotation |
| Repository pattern | Diesel with traits | Type-safe, testable |
| Config | dotenv + envy | Typed, validated config |
| RBAC query strategy | 2 queries (subquery, no JOINs) | Simplicity over performance |
| RBAC token strategy | Role + permissions in JWT | No DB lookup per request, refresh on login/refresh |
| RBAC endpoints | Deferred to M7.1 | No role CRUD endpoints needed yet |
| Default role on register | Always `user` | Simplest, no admin check at registration |

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
    async fn find_role_by_name(&self, name: &str) -> Result<Option<Role>>;
    async fn assign_role(&self, user_id: Uuid, role_id: Uuid) -> Result<()>;
    async fn get_user_role_ids(&self, user_id: Uuid) -> Result<Vec<Uuid>>;
    async fn get_permission_names_by_role_ids(&self, role_ids: &[Uuid]) -> Result<Vec<String>>;
}
// 2-query strategy: get_user_role_ids() → get_permission_names_by_role_ids()
// No JOINs — subquery in permissions lookup
```

## Adapters (Implementations)

| Port | Adapter | Location | Technology |
|---|---|---|---|
| `AuthProvider` | `LocalAuthProvider` | `src/auth/repository.rs` | bcrypt + jsonwebtoken |
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
         ├─ src/user/ports.rs  ←  src/user/repository.rs (adapter impl)
         └─ src/rbac/ports.rs  ←  src/rbac/repository.rs (adapter impl)

src/user/controller.rs
         ↓
src/user/service.rs
         ↓  (depends on traits)
src/user/ports.rs  ←  src/user/repository.rs (adapter impl)

src/rbac/service.rs
         ↓  (depends on traits)
src/rbac/ports.rs  ←  src/rbac/repository.rs (adapter impl)
```