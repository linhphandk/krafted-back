# M7 — Authorization Logic (RBAC)

**Deliverable: Roles and permissions are seeded in the database, default `user` role is assigned on registration, JWTs include role and permissions, auth middleware extracts role and permissions from token claims**

**Depends on: M6**

---

## Principle: RBAC in the Token, Not the Database Lookup

On every login and token refresh, the service loads the user's role and permissions from the database and embeds them in the JWT claims. This means the auth middleware can extract role and permissions from the token without a database lookup on every request. When role/permission data changes (e.g., admin assigns a new role), the user just logs in again to get an updated token.

The 2-query strategy avoids JOINs: first get the user's role IDs, then get the permission names for those role IDs using a subquery.

---

## Task 1 — Seed permissions and role_permissions (~30loc SQL)

**Files**: Add to `migrations/20260530150000_create_rbac/up.sql` or create a new migration

Seed data:
```sql
INSERT INTO permissions (name, description) VALUES
    ('users:read', 'View user profiles'),
    ('users:write', 'Create and update users'),
    ('roles:read', 'View roles and permissions'),
    ('roles:write', 'Create and update roles');

-- user role permissions: users:read
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'user' AND p.name = 'users:read';

-- admin role permissions: all
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'admin';
```

This gives:
- `user` role → `users:read`
- `admin` role → `users:read`, `users:write`, `roles:read`, `roles:write`

**Estimated lines**: ~30loc SQL

---

## Task 2 — Update JWT claims to include role and permissions (~30loc)

**File**: `src/auth/provider.rs`

Update the `Claims` struct and token generation to include role:

```rust
#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    email: String,
    role: String,
    exp: usize,
}
```

This is already implemented. The key change is ensuring that:
1. `register()` always sets `role: "user"` in the JWT
2. `login()` and `refresh_token()` pass the actual user role from `RbacService::get_user_permissions()`

These are already working in the current code — `AuthService::login()` fetches the role via `rbac_service.get_user_permissions(user.id)` and passes it to `auth_provider.login()` and `auth_provider.generate_access_token()`.

**Estimated lines**: ~0loc (already implemented, just verify)

---

## Task 3 — Update AuthenticatedUser to include permissions (~15loc)

**File**: `src/shared/types.rs`

Currently `AuthenticatedUser` has:
```rust
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
}
```

Update to include permissions:
```rust
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
    pub permissions: Vec<String>,
}
```

The auth middleware will need to be updated to also extract permissions. Since permissions are not yet in the JWT claims (only `role` is), the options are:

**Option A** (chosen): Load permissions from database in the middleware. This adds one DB query per request but keeps tokens small.

**Option B** (future): Add `permissions: Vec<String>` to JWT claims. This makes the token larger but avoids DB lookups.

For M7, we use Option A. The middleware calls `rbac_service.get_user_permissions(user.id)` after extracting `user` and `role` from the token.

**Estimated lines**: ~15loc

---

## Task 4 — Update auth middleware to load permissions (~25loc)

**File**: `src/shared/middleware.rs`

After extracting the user from the JWT, also load their permissions from the database:

```rust
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // ... extract token, get user ...
    let (user, role) = match state.auth_service.get_current_user(token.to_string()).await {
        Ok(u) => u,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response(),
    };

    let permissions = match state.rbac_service.get_user_permissions(user.id).await {
        Ok((_, perms)) => perms,
        Err(_) => vec![],
    };

    request.extensions_mut().insert(AuthenticatedUser {
        id: user.id,
        email: user.email,
        name: user.name,
        role,
        permissions,
    });

    next.run(request).await
}
```

This requires `AppState` to expose `rbac_service` or have a method to get permissions. Update `AppState` accordingly.

**Estimated lines**: ~25loc

---

## Task 5 — Update AppState to expose rbac_service (~10loc)

**File**: `src/state.rs`

Add `rbac_service` to `AppState`:

```rust
#[derive(Clone)]
pub struct AppState {
    pub auth_service: AuthService<LocalAuthProvider, DieselUserRepository, DieselSessionRepository>,
    pub rbac_service: Arc<RbacService>,
}
```

And update the constructor to store it.

**Estimated lines**: ~10loc

---

## Task 6 — RbacRepository expanded methods (verify existing) (~0loc)

**File**: `src/rbac/ports.rs`, `src/rbac/repository.rs`

The existing `RbacRepository` trait and `DieselRbacRepository` already implement:
- `find_role_by_name()` — find a role by name
- `assign_role()` — assign a role to a user
- `get_user_role_ids()` — get all role IDs for a user
- `get_permission_names_by_role_ids()` — get permission names for role IDs (uses subquery, no JOINs)

These are sufficient for M7. No new methods needed.

**Estimated lines**: ~0loc (already implemented)

---

## Task 7 — Assign default role on registration (verify existing) (~0loc)

**File**: `src/auth/service.rs`

`AuthService::register()` already calls `self.rbac_service.assign_default_role(user.id)?` after creating the user. This assigns the `user` role. No changes needed.

**Estimated lines**: ~0loc (already implemented)

---

## Task 8 — RBAC tests (~130loc)

**Files**: `tests/rbac_repository.rs` (expand), `tests/rbac_service.rs` (expand)

Repository integration tests:
- `test_find_role_by_name_user` — finds the `user` role
- `test_find_role_by_name_admin` — finds the `admin` role
- `test_find_role_by_name_not_found` — returns None
- `test_assign_role` — assigns a role to a user
- `test_get_user_role_ids` — gets role IDs for a user
- `test_get_permission_names_by_role_ids` — gets permissions for role IDs
- `test_get_permission_names_empty_ids` — returns empty vec for empty role IDs

Service unit tests:
- `test_assign_default_role_success` — assigns `user` role
- `test_assign_default_role_no_user_role` — returns Internal if `user` role not found
- `test_get_user_permissions_success` — returns role name + permissions
- `test_get_user_permissions_no_roles` — returns Internal error

**Estimated lines**: ~130loc

---

## Task 9 — Auth middleware RBAC tests (~80loc)

**File**: `tests/auth_e2e.rs` (expand)

Add E2E test cases:
- `test_register_assigns_user_role` — register a user, query `user_roles` table, verify `user` role assigned
- `test_login_jwt_contains_user_role` — login, decode JWT, verify `role: "user"` claim
- `test_refresh_token_preserves_role` — login, refresh, decode new JWT, verify role preserved
- `test_me_endpoint_returns_role` — login, GET /auth/me, verify response includes role in token

**Estimated lines**: ~80loc

---

## Verification

After completing all tasks, you should be able to:

1. `cargo test -- --test-threads=1` — all tests pass
2. Register a new user → `user` role automatically assigned
3. Login → JWT contains `role: "user"` and `sub` is the user UUID
4. `GET /auth/me` with valid token → `AuthenticatedUser` has `role` and `permissions` populated
5. `rbac_service.get_user_permissions(user_id)` → `("user", ["users:read"])` for a regular user
6. After admin assigns `admin` role → `get_user_permissions()` returns `("admin", ["users:read", "users:write", "roles:read", "roles:write"])`
7. Auth middleware extracts both `role` and `permissions` from database after JWT validation
8. No RBAC endpoints yet (no GET/POST for roles or permissions) — that's M12
9. No admin seed user yet — that's M12