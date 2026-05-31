# krafted-back — Implementation Plan

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
    mod.rs               # Re-exports + auth_router()
    controller.rs        # Axum handlers, routes, DTOs
    service.rs           # Auth business logic
    ports.rs             # AuthProvider trait
    provider.rs          # LocalAuthProvider adapter
    models.rs            # Auth domain models (Tokens, UserInfo)
  user/
    mod.rs
    models.rs            # User, NewUser, UpdateUser
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
  listing/
    mod.rs
    models.rs            # Listing, Category, DTOs, Pagination
    ports.rs             # ListingRepository, CategoryRepository traits
    repository.rs        # DieselListingRepository, DieselCategoryRepository adapters
    service.rs           # ListingService, CategoryService
    controller.rs        # Axum handlers for listing/category endpoints
  shared/
    errors.rs            # AppError, AppResult, ErrorResponse
    types.rs             # DbResult, AuthenticatedUser, PaginatedResponse
    config.rs            # Config struct (env-based)
    db.rs                 # establish_pool, run_migrations
    middleware.rs         # auth_middleware, admin_middleware
  main.rs
  lib.rs
  router.rs              # Compose all domain routers
  state.rs               # AppState (wiring)
  schema.rs              # Diesel-generated schema
  api_doc.rs             # utoipa OpenAPI spec
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

See `plan/` directory for detailed specs.

### Auth Template

- [x] [M1 — Project Scaffolding & Core Architecture](plan/M1_PROJECT_SCAFFOLDING_CORE_ARCHITECTURE.md)
- [x] [M2 — Database & Repository Layer](plan/M2_DATABASE_REPOSITORY_LAYER.md)
- [x] [M3 — Local Auth Implementation](plan/M3_LOCAL_AUTH_IMPLEMENTATION.md)
- [x] [M4 — Service Layer (Business Logic)](plan/M4_SERVICE_LAYER_BUSINESS_LOGIC.md)
- [x] [M5 — Controller Layer (Auth HTTP API)](plan/M5_CONTROLLER_LAYER_HTTP_API.md)
- [ ] [M6 — User Management API](plan/M6_USER_MANAGEMENT_API.md)
- [x] [M7 — Authorization Logic (RBAC)](plan/M7_AUTHORIZATION_LOGIC_RBAC.md)
- [ ] [M7.1 — Admin Seed & RBAC Endpoints](plan/M7.1_ADMIN_SEED_RBAC_ENDPOINTS.md)
- [ ] [M8 — Testing & Documentation](plan/M8_TESTING_DOCUMENTATION.md)

### Marketplace

- [ ] [M9 — Listing Domain: Migrations + Models + Schema](plan/M9_LISTINGS_MIGRATIONS_MODELS.md)
- [ ] [M10 — Listing Domain: Ports + Repositories](plan/M10_LISTINGS_PORTS_REPOSITORIES.md)
- [ ] [M11 — Listing Domain: Service Layer](plan/M11_LISTINGS_SERVICE.md)
- [ ] [M12 — Listing Domain: Controller Layer (HTTP API)](plan/M12_LISTINGS_CONTROLLERS.md)
- [ ] [M13 — Auth Middleware (JWT Extractor + Ownership Checks)](plan/M13_AUTH_MIDDLEWARE.md)

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
| RBAC token strategy | Role in JWT, permissions from DB | Small tokens, fresh permissions |
| RBAC endpoints | M7.1 | Deferred to separate milestone |
| Default role on register | Always `user` | Simplest, no admin check at registration |
| User management | Soft delete | `is_active = false`, not row removal |
| API documentation | utoipa + Scalar | Auto-generated from code, interactive UI |

## Adapters (Implementations)

| Port | Adapter | Location | Technology |
|---|---|---|---|
| `AuthProvider` | `LocalAuthProvider` | `src/auth/provider.rs` | bcrypt + jsonwebtoken |
| `UserRepository` | `DieselUserRepository` | `src/user/repository.rs` | Diesel + PostgreSQL |
| `SessionRepository` | `DieselSessionRepository` | `src/session/repository.rs` | Diesel + PostgreSQL |
| `RbacRepository` | `DieselRbacRepository` | `src/rbac/repository.rs` | Diesel + PostgreSQL |

## Dependency Graph

```
krafted-back:
  M1–M5 (done) → M7 (done) → M6 → M7.1 → M8
                               ↘
                                  M9 → M10 → M11 → M12 → M13

krafted-front:
  M1 (done) → M2 (done) → M3 (partial)
                            ↓
                            M4 → M5 → M6 → M7 (marketplace features)
                                             ↑
                                             requires lightweight RequireAuth (from M8)

                            M8 → M9 → M10 → M11 (auth template completion, deferred)
```

## Refresh Token Strategy (Source: Auth0)

- **Access tokens**: short-lived (minutes/hours), bearer tokens for API access
- **Refresh tokens**: longer-lived, used to obtain new access tokens without re-login
- **Refresh Token Rotation**: every refresh token exchange returns a NEW refresh token; old one is invalidated
- **Automatic Reuse Detection**: if a previously-used refresh token is sent again, invalidate the entire "token family" and require re-authentication
- **Token families**: track all refresh tokens descending from the original; reuse of any member invalidates all
- **Security**: treat all token holders as potentially malicious; rotation + reuse detection mitigates replay attacks
- **Implementation**: store refresh tokens in DB, rotate on every use, detect reuse, invalidate family on conflict
- **Token Delivery**: Return `{ access_token, refresh_token, expires_in }` in JSON response body. No cookies.