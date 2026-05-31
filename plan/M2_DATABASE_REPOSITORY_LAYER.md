# M2 — Database & Repository Layer

**Deliverable: Diesel migrations for all tables, domain models, port traits, and repository adapters — all compiling and tested against a real PostgreSQL database**

**Depends on: M1**

---

## Principle: Ports Define the Contract, Repositories Implement It

Each domain has a `ports.rs` file that defines a trait (the contract). The corresponding `repository.rs` implements that trait using Diesel queries against PostgreSQL. Services depend only on the trait — never on the concrete implementation. This makes services testable with mock repositories.

---

## Task 1 — Diesel migrations for users table (~30loc SQL)

**Files**: `migrations/20260530000000_create_users/up.sql`, `migrations/20260530000000_create_users/down.sql`

`up.sql`:
```sql
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

`down.sql`:
```sql
DROP TABLE users;
```

Also need a migration to add `password_hash`:
```sql
ALTER TABLE users ADD COLUMN password_hash VARCHAR(255) NOT NULL DEFAULT '';
```

The `DEFAULT ''` is temporary — password hashes are always set by the application. A subsequent migration or ALTER can make it `NOT NULL` without default after data is populated.

**Estimated lines**: ~15loc SQL

---

## Task 2 — Diesel migrations for sessions table (~20loc SQL)

**Files**: `migrations/20260530140000_create_sessions/up.sql`, `migrations/20260530140000_create_sessions/down.sql`

`up.sql`:
```sql
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token VARCHAR(255) NOT NULL UNIQUE,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_refresh_token ON sessions(refresh_token);
```

Indexes: `user_id` for lookups by user, `refresh_token` for token revocation and rotation lookups.

**Estimated lines**: ~15loc SQL

---

## Task 3 — Diesel migrations for RBAC tables (~40loc SQL)

**Files**: `migrations/20260530150000_create_rbac/up.sql`, `migrations/20260530150000_create_rbac/down.sql`

`up.sql`:
```sql
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) NOT NULL UNIQUE,
    description VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    description VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE user_roles (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE role_permissions (
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, permission_id)
);

-- Seed: default roles
INSERT INTO roles (name, description) VALUES ('user', 'Default role for all registered users');
INSERT INTO roles (name, description) VALUES ('admin', 'Administrator with full access');
```

Seed data creates `user` and `admin` roles. Permissions and `role_permissions` seeds come in M7.

`down.sql`:
```sql
DROP TABLE IF EXISTS role_permissions;
DROP TABLE IF EXISTS user_roles;
DROP TABLE IF EXISTS permissions;
DROP TABLE IF EXISTS roles;
```

**Estimated lines**: ~30loc SQL

---

## Task 4 — Domain models (~80loc)

**Files**: `src/user/models.rs`, `src/session/models.rs`, `src/rbac/models.rs`, `src/auth/models.rs`

Each domain defines Diesel models (Queryable/Insertable) that map to its database table.

`src/user/models.rs`:
```rust
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub password_hash: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser {
    pub email: String,
    pub name: String,
    pub password_hash: String,
}
```

`src/session/models.rs`:
```rust
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = crate::schema::sessions)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub refresh_token: String,
    pub expires_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::sessions)]
pub struct NewSession {
    pub user_id: Uuid,
    pub refresh_token: String,
    pub expires_at: NaiveDateTime,
}
```

`src/rbac/models.rs`:
```rust
use chrono::NaiveDateTime;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Selectable, Clone, Debug)]
#[diesel(table_name = crate::schema::roles)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::roles)]
pub struct NewRole {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Queryable, Selectable, Clone, Debug)]
#[diesel(table_name = crate::schema::permissions)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Selectable, Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::user_roles)]
pub struct UserRole {
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Selectable, Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::role_permissions)]
pub struct RolePermission {
    pub role_id: Uuid,
    pub permission_id: Uuid,
    pub created_at: NaiveDateTime,
}
```

`src/auth/models.rs`:
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
```

**Estimated lines**: ~80loc

---

## Task 5 — Port traits per domain (~50loc)

**Files**: `src/user/ports.rs`, `src/session/ports.rs`, `src/rbac/ports.rs`

Note: `src/auth/ports.rs` is defined in M3 since it's specific to the auth provider implementation.

`src/user/ports.rs`:
```rust
use async_trait::async_trait;
use uuid::Uuid;

use crate::shared::errors::AppResult;
use crate::user::models::{NewUser, User};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: NewUser) -> AppResult<User>;
    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>>;
}
```

`src/session/ports.rs`:
```rust
use async_trait::async_trait;

use crate::session::models::{NewSession, Session};
use crate::shared::errors::AppResult;

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: NewSession) -> AppResult<Session>;
    async fn find_by_token(&self, token: &str) -> AppResult<Option<Session>>;
    async fn revoke(&self, token: &str) -> AppResult<()>;
}
```

`src/rbac/ports.rs`:
```rust
use async_trait::async_trait;
use uuid::Uuid;

use crate::rbac::models::Role;
use crate::shared::errors::AppResult;

#[async_trait]
pub trait RbacRepository: Send + Sync {
    async fn find_role_by_name(&self, name: &str) -> AppResult<Option<Role>>;
    async fn assign_role(&self, user_id: Uuid, role_id: Uuid) -> AppResult<()>;
    async fn get_user_role_ids(&self, user_id: Uuid) -> AppResult<Vec<Uuid>>;
    async fn get_permission_names_by_role_ids(&self, role_ids: &[Uuid]) -> AppResult<Vec<String>>;
}
```

**Estimated lines**: ~50loc

---

## Task 6 — Repository adapters per domain (~200loc)

**Files**: `src/user/repository.rs`, `src/session/repository.rs`, `src/rbac/repository.rs`

Each adapter implements its port trait using Diesel queries. They all follow the same pattern: take a `DbPool` in the constructor, get a connection from the pool, execute the query, map errors to `AppError`.

`src/user/repository.rs` (~80loc):
- `DieselUserRepository::new(pool: DbPool)`
- `UserRepository::create()` — INSERT into users
- `UserRepository::find_by_email()` — SELECT from users WHERE email = $1
- `UserRepository::find_by_id()` — SELECT from users WHERE id = $1
- `map_diesel_error()` helper: maps `UniqueViolation` to `AppError::BadRequest("Email already exists")`

`src/session/repository.rs` (~65loc):
- `DieselSessionRepository::new(pool: DbPool)`
- `SessionRepository::create()` — INSERT into sessions, returns Session with generated UUID
- `SessionRepository::find_by_token()` — SELECT from sessions WHERE refresh_token = $1
- `SessionRepository::revoke()` — DELETE from sessions WHERE refresh_token = $1

`src/rbac/repository.rs` (~95loc):
- `DieselRbacRepository::new(pool: DbPool)`
- `RbacRepository::find_role_by_name()` — SELECT from roles WHERE name = $1
- `RbacRepository::assign_role()` — INSERT into user_roles
- `RbacRepository::get_user_role_ids()` — SELECT role_id from user_roles WHERE user_id = $1
- `RbacRepository::get_permission_names_by_role_ids()` — 2-query strategy: first get role_ids, then subquery to get permission names (no JOINs)

**Estimated lines**: ~240loc combined

---

## Task 7 — Integration tests for repositories (~200loc)

**Files**: `tests/user_repository.rs`, `tests/session_repository.rs`, `tests/rbac_repository.rs`

Each test file uses `testcontainers` to spin up a PostgreSQL container, run migrations, and test repository operations against a real database.

Pattern (all follow this structure):
```rust
use testcontainers::clients::Cli;
use testcontainers_modules::postgres::Postgres;
use krafted_back::shared::db::{establish_pool, run_migrations};
use krafted_back::user::repository::DieselUserRepository;
use krafted_back::user::ports::UserRepository;
use krafted_back::user::models::NewUser;

#[tokio::test]
async fn test_create_user() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let pool = establish_pool(&db_url, 4);
    run_migrations(&pool);

    let repo = DieselUserRepository::new(pool);
    let new_user = NewUser {
        email: "test@example.com".to_string(),
        name: "Test User".to_string(),
        password_hash: "hashed".to_string(),
    };
    let user = repo.create(new_user).await.unwrap();
    assert_eq!(user.email, "test@example.com");
}
```

Tests to write per domain:
- **User**: create, find_by_email (found/not found), find_by_id (found/not found), duplicate email
- **Session**: create, find_by_token (found/not found), revoke, revoke non-existent token
- **RBAC**: find_role_by_name (found/not found), assign_role, get_user_role_ids, get_permission_names_by_role_ids (empty/with data)

**Estimated lines**: ~200loc combined

---

## Task 8 — Update `src/schema.rs` via `diesel print-schema` (~80loc auto-generated)

**File**: `src/schema.rs` (auto-generated by `diesel print-schema`)

After all migrations are created, run:
```bash
diesel print-schema > src/schema.rs
```

This generates the schema module with table definitions. The file should declare all 6 tables:
- `users`
- `sessions`
- `roles`
- `permissions`
- `user_roles`
- `role_permissions`

Plus `joinable!` macros for foreign keys and `allow_tables_to_appear_in_same_query!`.

**Estimated lines**: ~80loc (auto-generated)

---

## Verification

After completing all tasks, you should be able to:

1. `cargo check` — compiles without errors
2. `diesel migration run` — all migrations apply cleanly
3. `cargo test -- --test-threads=1` — all repository integration tests pass
4. Each domain module has: `models.rs`, `ports.rs`, `repository.rs` (where applicable)
5. `schema.rs` contains all 6 tables with correct column definitions
6. Repository integration tests use testcontainers for real PostgreSQL
7. `SELECT * FROM roles` returns the two seeded rows (`user`, `admin`)