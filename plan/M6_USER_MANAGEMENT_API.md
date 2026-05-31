# M6 — User Management API

**Deliverable: Authenticated users can be listed (paginated), viewed, updated, and soft-deleted via REST API endpoints**

**Depends on: M5**

---

## Principle: CRUD Without Surprises

User management follows a predictable REST pattern. Every response uses the same `UserResponse` DTO. The `PATCH` endpoint supports partial updates — only fields present in the request body are modified. Soft delete sets `is_active = false` rather than removing the row.

---

## Task 1 — Add `is_active` column to users table (~15loc SQL)

**Files**: New migration `migrations/<timestamp>_add_is_active_to_users/up.sql`, `down.sql`

`up.sql`:
```sql
ALTER TABLE users ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT TRUE;
```

`down.sql`:
```sql
ALTER TABLE users DROP COLUMN is_active;
```

This enables soft delete — `DELETE /api/users/:id` sets `is_active = false` instead of removing the row.

Update `src/user/models.rs` to include the new field:
```rust
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub is_active: bool,        // new field
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub password_hash: String,
}
```

Update `src/schema.rs` by running `diesel print-schema`.

**Estimated lines**: ~15loc SQL + model update

---

## Task 2 — Expand UserRepository port with CRUD methods (~15loc)

**File**: `src/user/ports.rs`

Add methods to the `UserRepository` trait:

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: NewUser) -> AppResult<User>;
    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>>;
    async fn list(&self, page: i64, per_page: i64) -> AppResult<Vec<User>>;
    async fn count(&self) -> AppResult<i64>;
    async fn update(&self, id: Uuid, data: UpdateUser) -> AppResult<User>;
    async fn soft_delete(&self, id: Uuid) -> AppResult<()>;
}
```

New methods:
- `list(page, per_page)` — paginated user listing
- `count()` — total user count for pagination metadata
- `update(id, data)` — partial update using `UpdateUser` struct
- `soft_delete(id)` — sets `is_active = false`

**Estimated lines**: ~15loc

---

## Task 3 — Add UpdateUser model (~15loc)

**File**: `src/user/models.rs`

```rust
#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::users)]
pub struct UpdateUser {
    pub name: Option<String>,
    pub email: Option<String>,
    pub is_active: Option<bool>,
}
```

All fields are `Option<>` — only fields present in the PATCH request will be updated. Diesel's `AsChangeset` will set `NULL` fields to `DEFAULT` (no change) via the `changeset_options` attribute if needed, or we use `diesel::update(...).set(data)` which only updates non-None fields.

**Estimated lines**: ~15loc

---

## Task 4 — Expand DieselUserRepository with CRUD implementations (~80loc)

**File**: `src/user/repository.rs`

Add implementations for `list`, `count`, `update`, and `soft_delete`:

```rust
async fn list(&self, page: i64, per_page: i64) -> AppResult<Vec<User>> {
    let mut conn = self.pool.get().map_err(|_| AppError::Internal)?;
    users::table
        .filter(users::is_active.eq(true))
        .order(users::created_at.desc())
        .limit(per_page)
        .offset((page - 1) * per_page)
        .load::<User>(&mut conn)
        .map_err(|_| AppError::Internal)
}

async fn count(&self) -> AppResult<i64> {
    let mut conn = self.pool.get().map_err(|_| AppError::Internal)?;
    users::table
        .filter(users::is_active.eq(true))
        .count()
        .get_result::<i64>(&mut conn)
        .map_err(|_| AppError::Internal)
}

async fn update(&self, id: Uuid, data: UpdateUser) -> AppResult<User> {
    let mut conn = self.pool.get().map_err(|_| AppError::Internal)?;
    diesel::update(users::table.find(id))
        .set(data)
        .get_result::<User>(&mut conn)
        .map_err(|e| map_diesel_error(e, "User"))
}

async fn soft_delete(&self, id: Uuid) -> AppResult<()> {
    let mut conn = self.pool.get().map_err(|_| AppError::Internal)?;
    diesel::update(users::table.find(id))
        .set(users::is_active.eq(false))
        .execute(&mut conn)
        .map_err(|_| AppError::Internal)?;
    Ok(())
}
```

Note: `list()` only returns active users (`is_active = true`). Deactivated users are excluded by default.

**Estimated lines**: ~80loc (new methods)

---

## Task 5 — Expand UserService with CRUD methods (~30loc)

**File**: `src/user/service.rs`

```rust
pub async fn list(&self, page: i64, per_page: i64) -> AppResult<Vec<User>> {
    self.repo.list(page, per_page).await
}

pub async fn count(&self) -> AppResult<i64> {
    self.repo.count().await
}

pub async fn update(&self, id: Uuid, data: UpdateUser) -> AppResult<User> {
    self.repo.update(id, data).await
}

pub async fn soft_delete(&self, id: Uuid) -> AppResult<()> {
    self.repo.soft_delete(id).await
}
```

Each method is a thin delegation to the repository. Business logic (e.g., "don't let a user deactivate themselves") would be added here in the service layer, not in the controller.

**Estimated lines**: ~30loc (new methods)

---

## Task 6 — Pagination types in shared (~25loc)

**File**: `src/shared/types.rs` (additions)

Add generic pagination response and query parameter types:

```rust
#[derive(serde::Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(serde::Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl PaginationParams {
    pub fn normalize(self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(20).min(100).max(1);
        (page, per_page)
    }
}
```

Default: page=1, per_page=20, max per_page=100.

**Estimated lines**: ~25loc

---

## Task 7 — User controller (HTTP handlers) (~100loc)

**File**: `src/user/controller.rs` (new file)

```rust
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::shared::errors::AppResult;
use crate::shared::types::{AuthenticatedUser, PaginatedResponse, PaginationParams, UpdateUser};
use crate::state::AppState;

#[derive(serde::Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: String,
}

impl From<crate::user::models::User> for UserResponse {
    fn from(u: crate::user::models::User) -> Self {
        Self {
            id: u.id.to_string(),
            email: u.email,
            name: u.name,
            is_active: u.is_active,
            created_at: u.created_at.to_string(),
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub is_active: Option<bool>,
}
```

Handlers:
- **`list_users`** — `GET /api/users?page=&per_page=`, requires auth, returns `PaginatedResponse<UserResponse>`
- **`get_user`** — `GET /api/users/:id`, requires auth, returns `UserResponse`
- **`update_user`** — `PATCH /api/users/:id`, requires auth, takes `UpdateUserRequest`, returns `UserResponse`
- **`delete_user`** — `DELETE /api/users/:id`, requires auth, soft deletes (sets `is_active = false`), returns `200 OK`

**Estimated lines**: ~100loc

---

## Task 8 — User router module (~20loc)

**File**: `src/user/mod.rs` (update)

Add a `controller` module and `user_router()` function:

```rust
pub mod controller;
pub mod models;
pub mod ports;
pub mod repository;
pub mod service;

use crate::shared::middleware::auth_middleware;
use crate::state::AppState;
use axum::routing::{delete, get, patch};
use axum::Router;

pub fn user_router() -> axum::Router<AppState> {
    axum::Router::<AppState>::new()
        .route("/users", get(controller::list_users))
        .route("/users/:id", get(controller::get_user))
        .route("/users/:id", patch(controller::update_user))
        .route("/users/:id", delete(controller::delete_user))
        .layer(axum::middleware::from_fn_with_state(/* state */, auth_middleware))
}
```

All user routes are protected — auth middleware is applied to the entire router.

**Estimated lines**: ~20loc

---

## Task 9 — Wire user router into main router (~10loc)

**File**: `src/router.rs` (update)

Add the user router to `create_router()`:

```rust
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api-docs/openapi.json", get(openapi_json))
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .merge(auth_router(&state))
        .merge(crate::user::user_router())  // new
        .with_state(state)
}
```

Also add `pub mod user;` to `src/lib.rs` if not already present.

**Estimated lines**: ~10loc

---

## Task 10 — User controller integration tests (~150loc)

**File**: `tests/user_e2e.rs` (new file)

Tests using testcontainers, spinning up the full app:
- `test_list_users_empty` — GET /api/users with no users, returns empty list
- `test_list_users_paginated` — create 25 users, GET /api/users?page=2&per_page=10, verify pagination
- `test_get_user_by_id` — register user, GET /api/users/:id, verify user data
- `test_get_user_not_found` — GET /api/users/nonexistent-uuid, verify 404
- `test_update_user_name` — PATCH /api/users/:id with `{ "name": "New Name" }`, verify only name changed
- `test_update_user_email` — PATCH /api/users/:id with `{ "email": "new@example.com" }`
- `test_soft_delete_user` — DELETE /api/users/:id, verify is_active = false, verify user not in list
- `test_cannot_access_user_without_auth` — GET /api/users without Authorization, verify 401

**Estimated lines**: ~150loc

---

## Task 11 — Update OpenAPI spec with user endpoints (~15loc)

**File**: `src/api_doc.rs` (update)

Add user DTOs and paths to the OpenAPI spec:

```rust
components(
    schemas(
        // ... existing auth DTOs ...
        crate::user::controller::UserResponse,
        crate::user::controller::UpdateUserRequest,
        crate::shared::types::PaginatedResponse<UserResponse>,
    ),
),
paths(
    // ... existing auth paths ...
    crate::user::controller::list_users,
    crate::user::controller::get_user,
    crate::user::controller::update_user,
    crate::user::controller::delete_user,
),
tags(
    (name = "auth", description = "Authentication endpoints"),
    (name = "users", description = "User management endpoints"),
),
```

**Estimated lines**: ~15loc

---

## Verification

After completing all tasks, you should be able to:

1. `cargo test -- --test-threads=1` — all tests pass (including new user E2E tests)
2. `GET /api/users` — returns paginated list of active users (default: page=1, per_page=20)
3. `GET /api/users?page=2&per_page=10` — returns second page of 10 users
4. `GET /api/users/:id` — returns single user object
5. `PATCH /api/users/:id` — updates only provided fields
6. `DELETE /api/users/:id` — soft deletes user (sets `is_active = false`)
7. All user endpoints require valid Bearer token
8. `GET /scalar` — user endpoints appear in the interactive API docs
9. Soft-deleted users are excluded from list results
10. `UserResponse` DTO includes `is_active` field