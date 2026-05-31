# M12 — Auth Middleware

**Deliverable: JWT-based AuthUser extractor, protected listing endpoints enforce auth and ownership**

**Depends on: M11**

---

## Task 1 — Create `AuthUser` extractor

**Files**: `src/shared/middleware.rs`

Define:

```rust
#[derive(Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub email: String,
}
```

Implement `FromRequestParts<AppState>` for `AuthUser`:
1. Extract `Authorization` header from request
2. Parse `Bearer <token>` format
3. Decode JWT using `jwt_secret` from `AppState`
4. Extract `sub` (user_id as UUID) and `email` from claims
5. If any step fails, return `AppError::Unauthorized`
6. Return `AuthUser` on success

The JWT claims structure (from `LocalAuthProvider`):
```rust
struct Claims {
    sub: String,  // user_id as string
    email: String,
    exp: usize,
}
```

Use `jsonwebtoken::decode` with the same `jwt_secret` and `Header::default()` used in `LocalAuthProvider::generate_access_token()`.

Access `AppState` via `axum::extract::State` to get the JWT secret. The `FromRequestParts` implementation needs access to the state — use `axum::extract::FromRef` or pass secret through extensions.

**Simplest approach**: Store `jwt_secret` as an `Arc<String>` in `AppState`. In `FromRequestParts`, use `State<AppState>` to extract it.

---

## Task 2 — Update protected listing handlers to use `AuthUser`

**Files**: `src/listing/controller.rs`

Replace placeholder seller_id extraction with `auth_user: AuthUser` parameter on protected handlers:

- `create_listing(State(state), auth_user: AuthUser, Json(req))` — use `auth_user.user_id`
- `update_listing(State(state), auth_user: AuthUser, Path(id), Json(req))` — use `auth_user.user_id`
- `delete_listing(State(state), auth_user: AuthUser, Path(id))` — use `auth_user.user_id`
- `publish_listing(State(state), auth_user: AuthUser, Path(id))` — use `auth_user.user_id`
- `pause_listing(State(state), auth_user: AuthUser, Path(id))` — use `auth_user.user_id`
- `seller_listings(State(state), auth_user: AuthUser, Query(params))` — use `auth_user.user_id`

Public handlers (`list_listings`, `get_listing`, `list_categories`, `list_categories_by_kind`) remain unchanged — no `AuthUser` required.

---

## Task 3 — Wire auth middleware on protected routes

**Files**: `src/listing/mod.rs`

Split routes into public and protected groups:

```rust
use crate::shared::middleware::AuthUser;
use axum::middleware::from_fn;

pub fn listing_router() -> axum::Router<AppState> {
    let public_routes = axum::Router::<AppState>::new()
        .route("/api/listings", axum::routing::get(controller::list_listings))
        .route("/api/listings/{id}", axum::routing::get(controller::get_listing))
        .route("/api/categories", axum::routing::get(controller::list_categories));

    let protected_routes = axum::Router::<AppState>::new()
        .route("/api/listings", axum::routing::post(controller::create_listing))
        .route("/api/listings/mine", axum::routing::get(controller::seller_listings))
        .route("/api/listings/{id}", axum::routing::patch(controller::update_listing).delete(controller::delete_listing))
        .route("/api/listings/{id}/publish", axum::routing::post(controller::publish_listing))
        .route("/api/listings/{id}/pause", axum::routing::post(controller::pause_listing))
        .layer(from_fn(auth_middleware));

    public_routes.merge(protected_routes)
}
```

Note: Since `AuthUser` is extracted via `FromRequestParts`, the middleware layer approach ensures auth runs before the handler. Alternatively, since `FromRequestParts` automatically rejects unauthenticated requests with 401, we may not need an explicit middleware layer — the `AuthUser` extractor itself handles it. Use whichever approach is cleaner for axum.

---

## Task 4 — `cargo check` + manual API test

1. Start server
2. `GET /api/listings` — returns 200 (no auth required)
3. `GET /api/categories` — returns 200 (no auth required)
4. `POST /api/listings` without `Authorization` header — returns 401
5. `POST /api/listings` with valid JWT — returns 201
6. `PATCH /api/listings/:id` with different user's JWT — returns 403
7. `DELETE /api/listings/:id` with owner's JWT — returns 204

---

## Verification

1. `cargo check` passes
2. Public listing/category endpoints work without auth
3. Protected endpoints return 401 without valid JWT
4. Protected endpoints return 403 when seller_id doesn't match listing owner
5. Owner can create/update/delete/publish/pause their own listings