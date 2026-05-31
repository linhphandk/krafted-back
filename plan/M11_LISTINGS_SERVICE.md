# M11 — Listing Domain: Service Layer

**Deliverable: ListingService and CategoryService with business logic, unit tests**

**Depends on: M10**

---

## Task 1 — Define request/response DTOs

**Files**: `src/listing/models.rs`

Add to existing `models.rs`:

- `CreateListingRequest` — for controller deserialization:
  - `title: String`
  - `description: String`
  - `price_cents: i32`
  - `category_id: Uuid`
  - `condition: ListingCondition`
  - `quantity: i32` (default 1 if not provided — handle in service)

- `UpdateListingRequest` — all fields Optional:
  - `title: Option<String>`
  - `description: Option<String>`
  - `price_cents: Option<i32>`
  - `category_id: Option<Uuid>`
  - `status: Option<ListingStatus>`
  - `condition: Option<ListingCondition>`
  - `quantity: Option<i32>`

- `ListingResponse` — public DTO (serialize):
  - `id: String` (UUID as string)
  - `seller_id: String`
  - `title: String`
  - `description: String`
  - `price_cents: i32`
  - `category_id: String`
  - `category_name: Option<String>` (joined from category)
  - `status: String`
  - `condition: String`
  - `quantity: i32`
  - `created_at: String`
  - `updated_at: String`

- `PaginatedResponse<T>` — generic response wrapper:
  - `items: Vec<T>`
  - `total: i64`
  - `page: i64`
  - `per_page: i64`
  - `total_pages: i64`

---

## Task 2 — Create `ListingService`

**Files**: `src/listing/service.rs`

```rust
#[derive(Clone)]
pub struct ListingService<L: ListingRepository, C: CategoryRepository> {
    listing_repo: L,
    category_repo: C,
}
```

Methods:
- `create_listing(seller_id: Uuid, req: CreateListingRequest) -> AppResult<Listing>`:
  - Validate title not empty
  - Validate price_cents > 0
  - Verify category_id exists via `category_repo.find_by_id()`, return `BadRequest` if not
  - If quantity not provided (default), set to 1
  - Default status to `Draft`
  - Build `NewListing` and call `listing_repo.create()`

- `get_listing(id: Uuid) -> AppResult<Listing>`:
  - Call `listing_repo.find_by_id()`, return `NotFound` if None

- `list_listings(filters: ListingFilters, page: i64, per_page: i64) -> AppResult<PaginatedResult<Listing>>`:
  - For public browse: force `filters.status = Some(ListingStatus::Active)` to only show active listings
  - Delegate to `listing_repo.find_all()`

- `list_my_listings(seller_id: Uuid, page: i64, per_page: i64) -> AppResult<PaginatedResult<Listing>>`:
  - Delegate to `listing_repo.find_by_seller(seller_id, page, per_page)`

- `update_listing(id: Uuid, seller_id: Uuid, req: UpdateListingRequest) -> AppResult<Listing>`:
  - Find listing, verify `listing.seller_id == seller_id`, return `Forbidden` if not owner
  - If `category_id` provided, verify it exists
  - Call `listing_repo.update()`

- `delete_listing(id: Uuid, seller_id: Uuid) -> AppResult<()>`:
  - Find listing, verify `listing.seller_id == seller_id`, return `Forbidden` if not owner
  - Call `listing_repo.delete()`

- `publish_listing(id: Uuid, seller_id: Uuid) -> AppResult<Listing>`:
  - Find listing, verify ownership
  - Set `status = Active`, call `listing_repo.update()` with `UpdateListing { status: Some(Active), ..Default::default() }`

- `pause_listing(id: Uuid, seller_id: Uuid) -> AppResult<Listing>`:
  - Find listing, verify ownership
  - Set `status = Paused`, call `listing_repo.update()` with `UpdateListing { status: Some(Paused), ..Default::default() }`

---

## Task 3 — Create `CategoryService`

**Files**: `src/listing/service.rs` (same file)

```rust
#[derive(Clone)]
pub struct CategoryService<C: CategoryRepository> {
    category_repo: C,
}
```

Methods:
- `list_categories() -> AppResult<Vec<Category>>` — delegate to `category_repo.find_all()`
- `list_categories_by_kind(kind: &str) -> AppResult<Vec<Category>>` — delegate to `category_repo.find_by_kind(kind)`

---

## Task 4 — Update `src/listing/mod.rs`

**Files**: `src/listing/mod.rs`

```rust
pub mod models;
pub mod ports;
pub mod repository;
pub mod service;
```

---

## Task 5 — Unit tests for `ListingService`

**Files**: `src/listing/service.rs` (inside `#[cfg(test)] mod tests`)

Use `mockall` to mock `ListingRepository` and `CategoryRepository` (existing dependency in Cargo.toml).

Tests:
- `create_listing` validates title not empty → `BadRequest`
- `create_listing` validates price > 0 → `BadRequest`
- `create_listing` validates category exists → `BadRequest` if not found
- `create_listing` defaults status to Draft
- `get_listing` returns `NotFound` when listing doesn't exist
- `update_listing` returns `Forbidden` when seller_id doesn't match
- `delete_listing` returns `Forbidden` when seller_id doesn't match
- `publish_listing` changes status to Active
- `pause_listing` changes status to Paused
- `list_listings` forces status filter to Active for public browse

---

## Task 6 — `cargo check` + `cargo test`

Verify compilation and all unit tests pass.

---

## Verification

1. `cargo check` passes
2. Unit tests pass: `cargo test --lib listing`
3. `ListingService` methods enforce business rules (validation, ownership, status transitions)
4. `CategoryService` methods delegate to repository