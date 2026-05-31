# krafted-back — Implementation Plan

## Stack
- **Language**: Rust
- **Framework**: Axum
- **ORM**: Diesel
- **Database**: PostgreSQL
- **Identity Provider**: Local Auth (bcrypt + JWT)

## Architecture

Ports & Adapters (Hexagonal), Domain-Scoped. Each domain module is self-contained: controller → service → ports ← repository. See `plan/` directory for per-milestone details.

Key modules: `auth/`, `user/`, `rbac/`, `session/`, `listing/`, `shared/`

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

## Dependency Order

```
krafted-back:
  M1–M5 (done) → M7 (done) → M6 → M7.1 → M8
                               ↘
                                  M9 → M10 → M11 → M12 → M13
```