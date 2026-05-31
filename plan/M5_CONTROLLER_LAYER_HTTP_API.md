# M5 — Controller Layer (Auth HTTP API)

**Deliverable: Auth endpoints (register, login, logout, refresh, me) respond to HTTP requests with proper status codes, JSON bodies, and OpenAPI documentation**

**Depends on: M4**

---

## Principle: Controllers Are Thin, Services Do the Work

Controllers (Axum handlers) are responsible for three things:
1. **Parse and validate** the incoming HTTP request into domain types
2. **Delegate** to the appropriate service method
3. **Map** domain results/errors to HTTP responses

No business logic lives in controllers. If a handler needs to make a decision, it should call a service method.

---

## Task 1 — Auth request/response DTOs (~70loc)

**File**: `src/auth/controller.rs` (top section)

```rust
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "John Doe")]
    pub name: String,
    #[schema(example = "securepassword123")]
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "securepassword123")]
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(serde::Serialize, ToSchema)]
pub struct RegisterResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub expires_in: u64,
}

#[derive(serde::Serialize, ToSchema)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(serde::Serialize, ToSchema)]
pub struct RefreshResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(serde::Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
}
```

Note: `RegisterResponse` does NOT contain `refresh_token` — on registration, the user is logged in immediately with just an access token. They get a refresh token on subsequent login. This is a simplified flow; a future enhancement may add refresh tokens to registration.

**Estimated lines**: ~70loc

---

## Task 2 — Auth handler functions (~125loc)

**File**: `src/auth/controller.rs` (handler section)

Each handler follows the pattern: parse `State` → parse `Json` body (if POST) → call `state.auth_service.method()` → map result to HTTP response.

```rust
#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "User registered", body = RegisterResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
    ),
    tag = "auth",
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<RegisterResponse>)> {
    let (user, tokens) = state.auth_service.register(req.email, req.name, req.password).await?;
    let response = RegisterResponse {
        user: UserResponse {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
        },
        access_token: tokens.access_token,
        expires_in: tokens.expires_in,
    };
    Ok((StatusCode::OK, Json(response)))
}
```

Handlers:
- **`register`** — `POST /auth/register`, returns `RegisterResponse` (user + access_token + expires_in, no refresh_token)
- **`login`** — `POST /auth/login`, returns `LoginResponse` (user + access_token + refresh_token + expires_in)
- **`logout`** — `POST /auth/logout`, takes `LogoutRequest` with `refresh_token`, returns `200 OK`
- **`refresh`** — `POST /auth/refresh`, takes `RefreshRequest` with `refresh_token`, returns `RefreshResponse` (user + new tokens + expires_in)
- **`me`** — `GET /auth/me`, requires auth middleware, returns `UserResponse`

**Estimated lines**: ~125loc

---

## Task 3 — Auth middleware (JWT guard) (~45loc)

**File**: `src/shared/middleware.rs`

The auth middleware extracts the `Authorization: Bearer <token>` header, validates the JWT via `AuthService::get_current_user()`, and injects an `AuthenticatedUser` into the request extensions.

```rust
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::shared::types::AuthenticatedUser;
use crate::state::AppState;

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_header = request.headers().get("Authorization").and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => return (StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header").into_response(),
    };

    let (user, role) = match state.auth_service.get_current_user(token.to_string()).await {
        Ok(u) => u,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response(),
    };

    request.extensions_mut().insert(AuthenticatedUser {
        id: user.id,
        email: user.email,
        name: user.name,
        role,
    });

    next.run(request).await
}
```

The `AuthenticatedUser` struct contains `id`, `email`, `name`, and `role` — all extracted from the JWT claims.

**Estimated lines**: ~45loc

---

## Task 4 — Auth router composition (~25loc)

**File**: `src/auth/mod.rs`

```rust
pub mod controller;
pub mod models;
pub mod ports;
pub mod provider;
pub mod service;

use crate::shared::middleware::auth_middleware;
use crate::state::AppState;
use axum::routing::{get, post};

pub fn auth_router(state: &AppState) -> axum::Router<AppState> {
    let protected = axum::Router::<AppState>::new()
        .route("/auth/me", get(controller::me))
        .layer(axum::middleware::from_fn_with_state(state.clone(), auth_middleware));

    axum::Router::<AppState>::new()
        .route("/auth/register", post(controller::register))
        .route("/auth/login", post(controller::login))
        .route("/auth/logout", post(controller::logout))
        .route("/auth/refresh", post(controller::refresh))
        .merge(protected)
}
```

Public routes: register, login, logout, refresh
Protected routes (require valid JWT): me

**Estimated lines**: ~25loc

---

## Task 5 — Main router composition (~30loc)

**File**: `src/router.rs`

```rust
use axum::routing::get;
use axum::Json;
use axum::Router;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::api_doc::ApiDoc;
use crate::auth::auth_router;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api-docs/openapi.json", get(openapi_json))
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .merge(auth_router(&state))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
```

**Estimated lines**: ~30loc

---

## Task 6 — OpenAPI documentation for auth endpoints (~35loc)

**File**: `src/api_doc.rs`

```rust
use utoipa::OpenApi;

use crate::auth::controller::{
    LoginRequest, LoginResponse, LogoutRequest, RefreshRequest, RefreshResponse, RegisterRequest,
    UserResponse,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::auth::controller::register,
        crate::auth::controller::login,
        crate::auth::controller::logout,
        crate::auth::controller::refresh,
        crate::auth::controller::me,
    ),
    components(
        schemas(
            RegisterRequest,
            LoginRequest,
            LogoutRequest,
            RefreshRequest,
            LoginResponse,
            RefreshResponse,
            UserResponse,
            crate::shared::errors::ErrorResponse,
        ),
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
    ),
)]
pub struct ApiDoc;
```

Accessible at:
- `GET /api-docs/openapi.json` — raw OpenAPI JSON
- `GET /scalar` — interactive Scalar UI

**Estimated lines**: ~35loc

---

## Task 7 — Auth E2E tests (~430loc)

**File**: `tests/auth_e2e.rs`

Integration tests that spin up a real PostgreSQL container via testcontainers, run the full Axum app, and test all endpoints end-to-end:

```rust
use axum::http::StatusCode;
use krafted_back::router::create_router;
use krafted_back::shared::db::{establish_pool, run_migrations};
use krafted_back::state::AppState;
use serde_json::json;
use testcontainers::clients::Cli;
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;

fn setup(docker: &Cli) -> (testcontainers::Container<'_, Postgres>, axum::Router) {
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let pool = establish_pool(&db_url, 4);
    run_migrations(&pool);
    let state = AppState::new(pool, "test-secret".to_string(), 15);
    let app = create_router(state);
    (container, app)
}
```

Test cases:
- `test_register_success` — POST /auth/register, verify 200 + user + access_token
- `test_register_empty_email` — verify 400
- `test_register_short_password` — verify 400
- `test_register_duplicate_email` — verify 400
- `test_login_success` — register then login, verify 200 + user + tokens
- `test_login_wrong_password` — verify 400
- `test_login_nonexistent_user` — verify 400
- `test_me_success` — register, get token, GET /auth/me, verify 200 + user
- `test_me_no_token` — GET /auth/me without Authorization, verify 401
- `test_me_invalid_token` — GET /auth/me with bad token, verify 401
- `test_logout_success` — register, login, logout with refresh_token, verify 200
- `test_refresh_token_success` — register, login, refresh token, verify 200 + new tokens
- `test_refresh_token_rotation` — after refresh, old refresh_token is invalid
- `test_refresh_token_invalid` — verify 400
- `test_health_check` — GET /health, verify "OK"

**Estimated lines**: ~430loc

---

## Verification

After completing all tasks, you should be able to:

1. `cargo test -- --test-threads=1` — all E2E and unit tests pass
2. `GET /health` returns `200 OK`
3. `POST /auth/register` with valid body returns `200` + `RegisterResponse`
4. `POST /auth/register` with duplicate email returns `400`
5. `POST /auth/login` with valid credentials returns `200` + `LoginResponse` with tokens
6. `POST /auth/login` with wrong password returns `400`
7. `POST /auth/logout` with valid refresh_token returns `200`
8. `POST /auth/refresh` with valid refresh_token returns `200` + new tokens (rotation)
9. `GET /auth/me` with valid Bearer token returns `200` + `UserResponse`
10. `GET /auth/me` without token returns `401`
11. `GET /scalar` renders the interactive API documentation
12. All response DTOs include `utoipa::ToSchema` annotations