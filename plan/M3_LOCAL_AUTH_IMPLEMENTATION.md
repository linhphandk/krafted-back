# M3 — Local Auth Implementation

**Deliverable: LocalAuthProvider registers users (bcrypt), logs in (bcrypt verify + JWT), introspects tokens (JWT decode), refresh tokens work with rotation, and revoke works via session deletion**

**Depends on: M2**

---

## Principle: Auth Provider Is a Swappable Adapter

The `AuthProvider` trait in `src/auth/ports.rs` defines the authentication contract. `LocalAuthProvider` in `src/auth/provider.rs` implements it using bcrypt for password hashing and jsonwebtoken for JWT creation/verification. In the future, this could be swapped for an `AuthentikAuthProvider` or any OAuth2/OIDC provider without changing any service code.

---

## Task 1 — Define AuthProvider port trait (~25loc)

**File**: `src/auth/ports.rs`

```rust
use async_trait::async_trait;

use crate::auth::models::{Tokens, UserInfo};
use crate::shared::errors::AppResult;

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn register(&self, email: &str, name: &str, password: &str) -> AppResult<(UserInfo, Tokens)>;
    async fn login(&self, email: &str, password: &str, password_hash: &str, role: &str) -> AppResult<(Tokens, UserInfo)>;
    async fn introspect_token(&self, token: &str) -> AppResult<UserInfo>;
    async fn refresh_token(&self, refresh_token: &str) -> AppResult<Tokens>;
    async fn revoke_token(&self, token: &str) -> AppResult<()>;
    async fn generate_access_token(&self, user_id: &str, email: &str, role: &str) -> AppResult<String>;
    fn token_expiry_seconds(&self) -> u64;
}
```

Key differences from original PLAN.md:
- `register()` returns `(UserInfo, Tokens)` — includes the password hash so the service can store it
- `login()` takes `password_hash` as a parameter — the service fetches the user first, then passes the stored hash for verification
- `generate_access_token()` is a separate method — used by `AuthService.refresh_token()` to mint new access tokens without re-authenticating
- `token_expiry_seconds()` — used by controllers to set `expires_in` in responses

**Estimated lines**: ~25loc

---

## Task 2 — Auth domain models (~30loc)

**File**: `src/auth/models.rs`

```rust
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub sub: String,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct OidcConfig {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub revocation_endpoint: String,
    pub jwks_uri: String,
}
```

`OidcConfig` is a placeholder for future OAuth2/OIDC support — not used by `LocalAuthProvider` but kept for when the `AuthentikAuthProvider` adapter is implemented.

**Estimated lines**: ~30loc

---

## Task 3 — LocalAuthProvider implementation (~180loc)

**File**: `src/auth/provider.rs`

`LocalAuthProvider` implements `AuthProvider` using:
- **bcrypt** — for password hashing (`bcrypt::hash`, `bcrypt::verify`)
- **jsonwebtoken** — for JWT creation and verification
- **uuid** — for generating user IDs during registration

```rust
use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::auth::models::{Tokens, UserInfo};
use crate::auth::ports::AuthProvider;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct LocalAuthProvider {
    jwt_secret: String,
    jwt_expiry_minutes: u64,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    email: String,
    role: String,
    exp: usize,
}
```

Key methods:
- **`register()`** — generates a UUID, hashes the password with bcrypt, creates a JWT with `sub`, `email`, `role: "user"`, returns `(UserInfo, Tokens)`. The `UserInfo.password_hash` is the bcrypt hash so the caller can store it.
- **`login()`** — verifies the password against the stored hash with `bcrypt::verify`, generates a JWT with claims from the user, returns `(Tokens, UserInfo)`. Returns `AppError::BadRequest("Invalid email or password")` on failure.
- **`introspect_token()`** — decodes and verifies the JWT, extracts claims into `UserInfo`. Returns `AppError::BadRequest("Invalid token")` on failure.
- **`refresh_token()`** — returns `AppError::NotImplemented` (refresh is handled by `AuthService` using sessions, not by the provider directly).
- **`revoke_token()`** — returns `AppError::NotImplemented` (revocation is handled by `AuthService` deleting sessions, not by the provider).
- **`generate_access_token()`** — creates a new JWT with the given claims. Used by `AuthService.refresh_token()` to mint new access tokens.
- **`token_expiry_seconds()`** — returns `jwt_expiry_minutes * 60`.

**Estimated lines**: ~180loc (including unit tests)

---

## Task 4 — Auth service with refresh token rotation (~170loc)

**File**: `src/auth/service.rs`

`AuthService` orchestrates all auth operations. It depends on three port traits: `AuthProvider`, `UserRepository`, and `SessionRepository`. It also uses `RbacService` (injected via `Arc`) for role assignment.

```rust
#[derive(Clone)]
pub struct AuthService<A: AuthProvider, R: UserRepository, S: SessionRepository> {
    auth_provider: A,
    user_service: UserService<R>,
    session_repo: S,
    refresh_token_expiry_days: i64,
    rbac_service: Arc<RbacService>,
}
```

Key methods:
- **`register(email, name, password)`** — validates input (non-empty email/name, password >= 8 chars), calls `auth_provider.register()` to hash password and get tokens, creates user in DB via `user_service`, assigns default `user` role via `rbac_service.assign_default_role()`, returns `(User, Tokens)`.
- **`login(email, password)`** — finds user by email, gets user's role/permissions via `rbac_service.get_user_permissions()`, verifies password via `auth_provider.login()`, creates a session (refresh token) in DB, returns `(User, Tokens)`.
- **`logout(refresh_token)`** — revokes the session by deleting from DB via `session_repo.revoke()`.
- **`refresh_token(refresh_token)`** — finds session by token, checks expiry, finds user, gets role, generates new access token via `auth_provider.generate_access_token()`, revokes old session, creates new session (rotation), returns `(User, Tokens)`.
- **`get_current_user(access_token)`** — introspects token via `auth_provider.introspect_token()`, finds user by email, returns `(User, role)`.

Refresh token rotation strategy (from Auth0):
1. On login: generate a UUID refresh token, store in `sessions` table with `user_id` and `expires_at`
2. On refresh: find session by token, verify not expired, generate new access token + new refresh token (new UUID), delete old session, insert new session
3. On logout: delete session by refresh token
4. Token families / reuse detection: future enhancement — for now, rotation is basic

**Estimated lines**: ~170loc

---

## Task 5 — Wire AuthService into AppState (~35loc)

**File**: `src/state.rs`

```rust
use crate::auth::provider::LocalAuthProvider;
use crate::rbac::repository::DieselRbacRepository;
use crate::rbac::service::RbacService;
use crate::session::repository::DieselSessionRepository;
use crate::shared::db::DbPool;
use crate::user::repository::DieselUserRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: crate::auth::service::AuthService<
        LocalAuthProvider,
        DieselUserRepository,
        DieselSessionRepository,
    >,
}

impl AppState {
    pub fn new(pool: DbPool, jwt_secret: String, jwt_expiry_minutes: u64) -> Self {
        let auth_provider = LocalAuthProvider::new(jwt_secret.clone(), jwt_expiry_minutes);
        let user_repo = DieselUserRepository::new(pool.clone());
        let session_repo = DieselSessionRepository::new(pool.clone());
        let rbac_repo = DieselRbacRepository::new(pool.clone());
        let rbac_service = Arc::new(RbacService::new(Arc::new(rbac_repo)));
        let auth_service = crate::auth::service::AuthService::new(
            auth_provider,
            user_repo,
            session_repo,
            7, // refresh_token_expiry_days
            rbac_service,
        );
        Self { auth_service }
    }
}
```

All dependencies are wired here. No service depends on concrete types — they all use trait bounds. The `pool` is cloned for each repository since `DbPool` is `Arc`-wrapped internally.

**Estimated lines**: ~35loc (current implementation)

---

## Task 6 — Auth service unit tests with mocks (~380loc)

**Files**: `tests/auth_service.rs`, `tests/auth_service_integration.rs`

`tests/auth_service.rs` — unit tests using `mockall` to mock `AuthProvider`, `UserRepository`, and `SessionRepository`:

```rust
use mockall::mock;

mock! {
    pub MockAuthProvider {}
    #[async_trait]
    impl AuthProvider for MockAuthProvider {
        async fn register(&self, email: &str, name: &str, password: &str) -> AppResult<(UserInfo, Tokens)>;
        async fn login(&self, email: &str, password: &str, password_hash: &str, role: &str) -> AppResult<(Tokens, UserInfo)>;
        async fn introspect_token(&self, token: &str) -> AppResult<UserInfo>;
        async fn refresh_token(&self, refresh_token: &str) -> AppResult<Tokens>;
        async fn revoke_token(&self, token: &str) -> AppResult<()>;
        async fn generate_access_token(&self, user_id: &str, email: &str, role: &str) -> AppResult<String>;
        fn token_expiry_seconds(&self) -> u64;
    }
}

mock! {
    pub MockUserRepository {}
    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn create(&self, user: NewUser) -> AppResult<User>;
        async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
        async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>>;
    }
}

mock! {
    pub MockSessionRepository {}
    #[async_trait]
    impl SessionRepository for MockSessionRepository {
        async fn create(&self, session: NewSession) -> AppResult<Session>;
        async fn find_by_token(&self, token: &str) -> AppResult<Option<Session>>;
        async fn revoke(&self, token: &str) -> AppResult<()>;
    }
}
```

Test cases:
- `test_register_success` — mocks auth_provider.register, user_repo.create, rbac_service.assign_default_role
- `test_register_empty_email` — returns BadRequest
- `test_register_short_password` — returns BadRequest
- `test_login_success` — mocks user lookup, rbac, auth_provider.login, session create
- `test_login_user_not_found` — returns BadRequest
- `test_login_wrong_password` — returns BadRequest
- `test_logout_success` — mocks session_repo.revoke
- `test_refresh_token_success` — mocks session lookup, revoke, create, token generation
- `test_refresh_token_expired` — returns BadRequest
- `test_refresh_token_invalid` — returns BadRequest
- `test_get_current_user_success` — introspects token, finds user

`tests/auth_service_integration.rs` — integration test with real LocalAuthProvider, real DB, and testcontainers:
- Register a user, verify tokens are returned
- Login with correct password, verify access token
- Login with wrong password, verify error

**Estimated lines**: ~380loc combined

---

## Task 7 — Auth provider unit tests (~100loc)

**File**: `src/auth/provider.rs` (inline `#[cfg(test)]` module)

Tests at the bottom of `provider.rs`:
- `test_register_hashes_password_and_returns_tokens` — create provider, call register, verify password hash starts with `$2b$`, verify access token is non-empty
- `test_login_valid_password` — hash a password, call login, verify tokens returned
- `test_login_wrong_password` — call login with wrong password, verify error
- `test_introspect_valid_token` — generate token, introspect, verify claims
- `test_introspect_expired_token` — create provider with 0 minute expiry, verify introspection fails

**Estimated lines**: ~100loc

---

## Verification

After completing all tasks, you should be able to:

1. `cargo test -- --test-threads=1` — all unit and integration tests pass
2. `LocalAuthProvider::register()` hashes the password with bcrypt and returns a JWT
3. `LocalAuthProvider::login()` verifies a password against a stored bcrypt hash
4. `LocalAuthProvider::introspect_token()` decodes and validates JWTs
5. `AuthService::register()` creates a user, assigns the `user` role, and returns tokens
6. `AuthService::login()` finds the user, verifies password, creates a session
7. `AuthService::refresh_token()` rotates the refresh token (old revoked, new created)
8. `AuthService::logout()` revokes the session
9. All tests use mockall for service layer tests and testcontainers for integration tests