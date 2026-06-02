# M14 — Admin Seed & RBAC Endpoints

**Deliverable: bootstrap admin user on startup, expose RBAC management endpoints**

**Depends on: M13**

---

## Principle: First Admin Is Bootstrapped, Then Admins Manage Roles

The admin user is created on server startup from environment variables (`ADMIN_EMAIL`, `ADMIN_PASSWORD`). After that, only users with the `admin` role can access RBAC management endpoints. The frontend uses these endpoints for the role management UI.

---

## Task 1 — Seed admin user on startup (~60loc)

**File**: `src/main.rs` (update)

After running migrations and creating `AppState`, call a function to seed the admin user if it doesn't exist:

```rust
use krafted_back::auth::service::AuthService;
use krafted_back::rbac::service::RbacService;

async fn seed_admin_user(state: &AppState) {
    let admin_email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@example.com".to_string());
    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "adminpassword".to_string());
    let admin_name = std::env::var("ADMIN_NAME").unwrap_or_else(|_| "Admin".to_string());

    // Check if admin user exists
    if let Ok(Some(_)) = state.auth_service.find_user_by_email(&admin_email).await {
        tracing::info!("Admin user already exists, skipping seed");
        return;
    }

    match state.auth_service.register(
        admin_email.clone(),
        admin_name,
        admin_password,
    ).await {
        Ok((user, _tokens)) => {
            // Assign admin role
            if let Err(e) = state.rbac_service.assign_admin_role(user.id).await {
                tracing::error!("Failed to assign admin role: {:?}", e);
            } else {
                tracing::info!("Admin user seeded: {}", admin_email);
            }
        }
        Err(e) => {
            tracing::error!("Failed to seed admin user: {:?}", e);
        }
    }
}
```

Environment variables:
- `ADMIN_EMAIL` — admin user email (default: `admin@example.com`)
- `ADMIN_PASSWORD` — admin user password (default: `adminpassword`)
- `ADMIN_NAME` — admin display name (default: `Admin`)

**Estimated lines**: ~60loc

---

## Task 2 — Add assign_admin_role to RbacService (~15loc)

**File**: `src/rbac/service.rs`

```rust
pub async fn assign_admin_role(&self, user_id: Uuid) -> AppResult<()> {
    let role = self.repo.find_role_by_name("admin").await?;
    let role = role.ok_or(AppError::Internal)?;
    self.repo.assign_role(user_id, role.id).await
}
```

Similar to `assign_default_role()` but assigns the `admin` role.

**Estimated lines**: ~15loc

---

## Task 3 — Add find_user_by_email to AuthService (~10loc)

**File**: `src/auth/service.rs`

Expose `find_user_by_email()` so the seed function can check if the admin user exists:

```rust
pub async fn find_user_by_email(&self, email: &str) -> AppResult<Option<User>> {
    self.user_service.find_by_email(email).await
}
```

**Estimated lines**: ~5loc

---

## Task 4 — RBAC management endpoints (~120loc)

**File**: `src/rbac/controller.rs` (new file)

```rust
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::shared::errors::AppResult;
use crate::shared::types::AuthenticatedUser;
use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub struct RoleResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct AssignRoleRequest {
    pub role_id: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RevokeRoleRequest {
    pub role_id: String,
}
```

Handlers:
- **`list_roles`** — `GET /api/roles`, requires auth, returns `Vec<RoleResponse>` with permissions for each role
- **`assign_role`** — `POST /api/users/:id/roles`, requires auth + admin, takes `AssignRoleRequest`, assigns role to user
- **`revoke_role`** — `DELETE /api/users/:id/roles/:role_id`, requires auth + admin, revokes role from user

**Estimated lines**: ~120loc

---

## Task 5 — RBAC router module (~25loc)

**File**: `src/rbac/mod.rs` (update)

Add `controller` module and `rbac_router()`:

```rust
pub mod controller;
pub mod models;
pub mod ports;
pub mod repository;
pub mod service;

use crate::shared::middleware::auth_middleware;
use crate::state::AppState;
use axum::routing::{delete, get, post};
use axum::Router;

pub fn rbac_router(state: &AppState) -> axum::Router<AppState> {
    axum::Router::<AppState>::new()
        .route("/roles", get(controller::list_roles))
        .route("/users/:id/roles", post(controller::assign_role))
        .route("/users/:id/roles/:role_id", delete(controller::revoke_role))
        .layer(axum::middleware::from_fn_with_state(state.clone(), auth_middleware))
}
```

**Estimated lines**: ~25loc

---

## Task 6 — Admin-only middleware guard (~30loc)

**File**: `src/shared/middleware.rs` (addition)

```rust
pub async fn admin_middleware(
    Extension(user): Extension<AuthenticatedUser>,
    request: Request,
    next: Next,
) -> Response {
    if user.role != "admin" {
        return (StatusCode::FORBIDDEN, "Admin access required").into_response();
    }
    next.run(request).await
}
```

Update the `rbac_router()` to apply `admin_middleware` for role assignment and revocation endpoints:

```rust
let admin_routes = axum::Router::<AppState>::new()
    .route("/users/:id/roles", post(controller::assign_role))
    .route("/users/:id/roles/:role_id", delete(controller::revoke_role))
    .layer(axum::middleware::from_fn(admin_middleware));

axum::Router::<AppState>::new()
    .route("/roles", get(controller::list_roles))
    .merge(admin_routes)
    .layer(axum::middleware::from_fn_with_state(state.clone(), auth_middleware))
```

`GET /api/roles` is available to any authenticated user. Role assignment/revocation requires admin.

**Estimated lines**: ~30loc

---

## Task 7 — Expand RbacRepository with additional methods (~30loc)

**File**: `src/rbac/ports.rs`, `src/rbac/repository.rs`

Add methods needed by the RBAC controller:

```rust
async fn find_role_by_id(&self, id: Uuid) -> AppResult<Option<Role>>;
async fn revoke_role(&self, user_id: Uuid, role_id: Uuid) -> AppResult<()>;
async fn list_roles(&self) -> AppResult<Vec<Role>>;
```

Implementations:
- `find_role_by_id()` — SELECT from roles WHERE id = $1
- `revoke_role()` — DELETE from user_roles WHERE user_id = $1 AND role_id = $2
- `list_roles()` — SELECT all roles

**Estimated lines**: ~30loc

---

## Task 8 — Expand RbacService with controller methods (~25loc)

**File**: `src/rbac/service.rs`

```rust
pub async fn list_roles(&self) -> AppResult<Vec<Role>> {
    // list all roles with their permissions
}

pub async fn assign_role_by_id(&self, user_id: Uuid, role_id: Uuid) -> AppResult<()> {
    let role = self.repo.find_role_by_id(role_id).await?;
    if role.is_none() {
        return Err(AppError::BadRequest("Role not found".to_string()));
    }
    self.repo.assign_role(user_id, role_id).await
}

pub async fn revoke_role_by_id(&self, user_id: Uuid, role_id: Uuid) -> AppResult<()> {
    self.repo.revoke_role(user_id, role_id).await
}
```

**Estimated lines**: ~25loc

---

## Task 9 — Wire RBAC router into main router (~10loc)

**File**: `src/router.rs` (update)

```rust
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api-docs/openapi.json", get(openapi_json))
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .merge(auth_router(&state))
        .merge(crate::user::user_router())
        .merge(crate::rbac::rbac_router(&state))
        .with_state(state)
}
```

**Estimated lines**: ~5loc

---

## Task 10 — RBAC endpoint E2E tests (~150loc)

**File**: `tests/rbac_e2e.rs` (new file)

- `test_list_roles` — GET /api/roles, verify `user` and `admin` roles returned
- `test_list_roles_requires_auth` — GET /api/roles without token, verify 401
- `test_assign_role_as_admin` — admin assigns a role to a user, verify 200
- `test_assign_role_as_non_admin` — regular user tries to assign role, verify 403
- `test_revoke_role_as_admin` — admin revokes a role, verify 200
- `test_revoke_role_as_non_admin` — regular user tries to revoke role, verify 403
- `test_assign_nonexistent_role` — verify 400
- `test_admin_seed_on_startup` — verify admin user exists after app starts

**Estimated lines**: ~150loc

---

## Task 11 — Update OpenAPI spec (~15loc)

**File**: `src/api_doc.rs` (update)

Add RBAC DTOs and paths:

```rust
components(schemas(
    // ... existing ...
    crate::rbac::controller::RoleResponse,
    crate::rbac::controller::AssignRoleRequest,
    crate::rbac::controller::RevokeRoleRequest,
)),
paths(
    // ... existing ...
    crate::rbac::controller::list_roles,
    crate::rbac::controller::assign_role,
    crate::rbac::controller::revoke_role,
),
tags(
    (name = "auth", description = "Authentication endpoints"),
    (name = "users", description = "User management endpoints"),
    (name = "rbac", description = "Role and permission management endpoints"),
),
```

**Estimated lines**: ~15loc

---

## Task 12 — Update .env.example (~5loc)

**File**: `.env.example` (update)

Add admin seed variables:
```
ADMIN_EMAIL=admin@example.com
ADMIN_PASSWORD=adminpassword
ADMIN_NAME=Admin
```

**Estimated lines**: ~5loc

---

## Verification

After completing all tasks, you should be able to:

1. `cargo test -- --test-threads=1` — all tests pass
2. Server startup seeds admin user with email from `ADMIN_EMAIL` env var
3. Admin user has `admin` role with all permissions
4. `GET /api/roles` — returns list of roles with their permissions
5. `POST /api/users/:id/roles` — admin can assign roles
6. `DELETE /api/users/:id/roles/:role_id` — admin can revoke roles
7. Regular users get 403 Forbidden on role management endpoints
8. `GET /scalar` — RBAC endpoints appear in the interactive API docs
9. Admin user can be verified by logging in with `ADMIN_EMAIL` / `ADMIN_PASSWORD`