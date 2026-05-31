# M4 — Service Layer (Business Logic)

**Deliverable: UserService and RbacService implement all business logic, depend only on port traits, and are fully tested**

**Depends on: M3**

---

## Principle: Services Depend on Traits, Never on Concrete Adapters

Service structs are generic over their repository trait: `UserService<R: UserRepository>`, `RbacService` wraps `Arc<dyn RbacRepository>`. This means services can be instantiated with mock repositories in tests and real Diesel repositories in production. The `AppState` wires the concrete types together.

---

## Task 1 — UserService (~30loc)

**File**: `src/user/service.rs`

`UserService` provides CRUD operations on users. It's generic over `UserRepository` and delegates all database operations to the repository.

```rust
use uuid::Uuid;

use crate::shared::errors::AppResult;
use crate::user::models::{NewUser, User};
use crate::user::ports::UserRepository;

#[derive(Clone)]
pub struct UserService<R: UserRepository> {
    repo: R,
}

impl<R: UserRepository> UserService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn create(&self, new_user: NewUser) -> AppResult<User> {
        self.repo.create(new_user).await
    }

    pub async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        self.repo.find_by_email(email).await
    }

    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        self.repo.find_by_id(id).await
    }
}
```

Note: Additional methods (`update`, `delete`, `list` with pagination) will be added in M6 when the user management API is built.

**Estimated lines**: ~30loc

---

## Task 2 — RbacService (~40loc)

**File**: `src/rbac/service.rs`

`RbacService` handles role assignment and permission lookup. It uses `Arc<dyn RbacRepository>` rather than generic parameters because it's shared across multiple services (injected into `AuthService`).

```rust
use std::sync::Arc;
use uuid::Uuid;

use crate::rbac::ports::RbacRepository;
use crate::shared::errors::{AppError, AppResult};

pub struct RbacService {
    repo: Arc<dyn RbacRepository>,
}

impl RbacService {
    pub fn new(repo: Arc<dyn RbacRepository>) -> Self {
        Self { repo }
    }

    pub async fn assign_default_role(&self, user_id: Uuid) -> AppResult<()> {
        let role = self.repo.find_role_by_name("user").await?;
        let role = role.ok_or(AppError::Internal)?;
        self.repo.assign_role(user_id, role.id).await
    }

    pub async fn get_user_permissions(
        &self,
        user_id: Uuid,
    ) -> AppResult<(String, Vec<String>)> {
        let role_ids = self.repo.get_user_role_ids(user_id).await?;
        if role_ids.is_empty() {
            return Err(AppError::Internal);
        }
        let permissions = self.repo.get_permission_names_by_role_ids(&role_ids).await?;
        let role = self.repo.find_role_by_name("user").await?.ok_or(AppError::Internal)?;
        Ok((role.name, permissions))
    }
}
```

Key design decisions:
- **`assign_default_role()`** — always assigns the `user` role on registration. This is called by `AuthService::register()`.
- **`get_user_permissions()`** — uses the 2-query strategy: first get role IDs for the user, then get permission names for those role IDs. Returns the role name and permissions list.
- **Currently returns only the first role name** — when a user has multiple roles, `get_user_permissions()` returns the `user` role name hard-coded. This will be replaced with proper multi-role support in M7.

**Estimated lines**: ~40loc

---

## Task 3 — AuthService orchestration (defined in M3, summarized here)

**File**: `src/auth/service.rs`

`AuthService` is the main orchestrator. It depends on:
- `AuthProvider` — for password hashing and JWT operations
- `UserRepository` (via `UserService`) — for user storage
- `SessionRepository` — for refresh token storage
- `RbacService` (via `Arc`) — for role assignment

Key methods (from M3):
- `register()` — validates input, hashes password, creates user, assigns default role
- `login()` — finds user, verifies password, creates session, returns tokens
- `logout()` — revokes session
- `refresh_token()` — validates session, rotates token
- `get_current_user()` — introspects JWT, returns user

**Estimated lines**: ~170loc (from M3)

---

## Task 4 — User service unit tests (~75loc)

**File**: `tests/user_service.rs`

```rust
use krafted_back::user::models::{NewUser, User};
use krafted_back::user::ports::UserRepository;
use krafted_back::user::service::UserService;
use krafted_back::shared::errors::AppResult;
use async_trait::async_trait;
use uuid::Uuid;
use chrono::NaiveDateTime;
use mockall::mock;

mock! {
    pub MockUserRepo {}
    #[async_trait]
    impl UserRepository for MockUserRepo {
        async fn create(&self, user: NewUser) -> AppResult<User>;
        async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
        async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>>;
    }
}
```

Tests:
- `test_create_user` — mock repo returns user, verify service returns it
- `test_find_by_email_found` — mock repo returns Some(user), verify service returns it
- `test_find_by_email_not_found` — mock repo returns None, verify service returns None
- `test_find_by_id_found` — mock repo returns Some(user)
- `test_find_by_id_not_found` — mock repo returns None

**Estimated lines**: ~75loc

---

## Task 5 — RBAC service unit tests (~80loc)

**File**: `tests/rbac_service.rs`

Tests using mock `RbacRepository`:
- `test_assign_default_role` — mocks `find_role_by_name("user")` returning the default role, mocks `assign_role`
- `test_assign_default_role_no_role` — `find_role_by_name` returns None, verify Internal error
- `test_get_user_permissions` — mocks `get_user_role_ids`, `get_permission_names_by_role_ids`, `find_role_by_name`
- `test_get_user_permissions_no_roles` — `get_user_role_ids` returns empty, verify Internal error

**Estimated lines**: ~80loc

---

## Verification

After completing all tasks, you should be able to:

1. `cargo test -- --test-threads=1` — all service unit tests pass
2. `UserService` is generic over `UserRepository` — can be instantiated with mock or real repo
3. `RbacService` uses `Arc<dyn RbacRepository>` — can be shared across services
4. `AuthService` orchestrates register, login, logout, refresh — all tested with mocks
5. No service depends on concrete repository types — only port traits
6. `RbacService::assign_default_role()` correctly assigns the `user` role
7. `RbacService::get_user_permissions()` returns role name + permissions list