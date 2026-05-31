# M9 — Listing Domain: Ports + Repositories

**Deliverable: CategoryRepository, ListingRepository traits and Diesel implementations, integration tests**

**Depends on: M8**

---

## Task 1 — Define port traits

**Files**: `src/listing/ports.rs`

`CategoryRepository` trait (Send + Sync, async_trait):
- `async fn find_all(&self) -> AppResult<Vec<Category>>`
- `async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Category>>`
- `async fn find_by_kind(&self, kind: &str) -> AppResult<Vec<Category>>`

`ListingRepository` trait (Send + Sync, async_trait):
- `async fn create(&self, listing: NewListing) -> AppResult<Listing>`
- `async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Listing>>`
- `async fn find_all(&self, filters: ListingFilters, page: i64, per_page: i64) -> AppResult<PaginatedResult<Listing>>`
- `async fn find_by_seller(&self, seller_id: Uuid, page: i64, per_page: i64) -> AppResult<PaginatedResult<Listing>>`
- `async fn update(&self, id: Uuid, data: UpdateListing) -> AppResult<Listing>`
- `async fn delete(&self, id: Uuid) -> AppResult<()>`
- `async fn count_by_seller(&self, seller_id: Uuid) -> AppResult<i64>`

---

## Task 2 — Create repository adapter structs

**Files**: `src/listing/repository.rs`

- `DieselCategoryRepository { pool: DbPool }` — `Clone`, `new(pool: DbPool)`
- `DieselListingRepository { pool: DbPool }` — `Clone`, `new(pool: DbPool)`

---

## Task 3 — Implement `DieselCategoryRepository`

**Files**: `src/listing/repository.rs`

- `find_all` — `categories::table.load::<Category>(&mut conn)`
- `find_by_id` — `categories::table.find(id).first::<Category>(&mut conn).optional()`
- `find_by_kind` — `categories::table.filter(categories::kind.eq(kind)).load::<Category>(&mut conn)`

Use the `map_diesel_error` pattern from `user/repository.rs`.

---

## Task 4 — Implement `DieselListingRepository`

**Files**: `src/listing/repository.rs`

`create`:
- `diesel::insert_into(listings::table).values(&listing).get_result::<Listing>(&mut conn)`

`find_by_id`:
- `listings::table.find(id).first::<Listing>(&mut conn).optional()`

`find_all` — dynamic query builder from `ListingFilters`:
1. Start with `listings::table.into_boxed()`
2. Apply `.filter(listings::status.eq(...))` when `filters.status` is `Some`
3. Apply `.filter(listings::category_id.eq(...))` when `filters.category_id` is `Some`
4. Apply `.filter(listings::title.ilike(format!("%{}%", search)))` when `filters.search` is `Some`
5. When `filters.kind` is `Some`, join with `categories` table and filter `categories::kind.eq(kind)`. Use `listings::table.inner_join(categories::table.on(listings::category_id.eq(categories::id)))`.
6. Apply sorting: `Newest` → `.order(listings::created_at.desc())`, `PriceAsc` → `.order(listings::price_cents.asc())`, `PriceDesc` → `.order(listings::price_cents.desc())`
7. Count total matching rows with a separate `SELECT COUNT(*)` query (same filters, no pagination)
8. Apply `.offset((page - 1) * per_page).limit(per_page)`
9. Return `PaginatedResult { items, total, page, per_page }`

`find_by_seller`:
- Filter by `seller_id`, paginate, return `PaginatedResult`

`update`:
- `diesel::update(listings::table.find(id)).set(data).get_result::<Listing>(&mut conn)`
- If 0 rows affected, return `AppError::NotFound("Listing not found")`

`delete`:
- `diesel::delete(listings::table.find(id)).execute(&mut conn)`
- If 0 rows affected, return `AppError::NotFound("Listing not found")`

`count_by_seller`:
- `listings::table.filter(listings::seller_id.eq(seller_id)).count().get_result::<i64>(&mut conn)`

---

## Task 5 — Update `src/listing/mod.rs`

**Files**: `src/listing/mod.rs`

```rust
pub mod models;
pub mod ports;
pub mod repository;
```

---

## Task 6 — Write integration tests

**Files**: `tests/listing_repository.rs`

Use testcontainers (existing pattern from `tests/user_repository.rs`).

Test `DieselCategoryRepository`:
- `find_all` returns seeded categories
- `find_by_id` returns a category
- `find_by_kind("craft")` returns only craft categories
- `find_by_kind("supply")` returns only supply categories

Test `DieselListingRepository`:
- `create` inserts a listing
- `find_by_id` returns the listing
- `find_all` with default filters returns active listings
- `find_all` with status=draft returns draft listings
- `find_all` with kind=craft returns only craft listings
- `find_all` with search returns matching listings
- `find_by_seller` returns seller's listings
- `update` changes listing fields
- `delete` removes listing
- `count_by_seller` counts seller's listings

---

## Task 7 — `cargo check` + `cargo test`

Verify all code compiles and tests pass.

---

## Verification

1. `cargo check` passes
2. `cargo test` (or `cargo test --tests listing_repository`) passes
3. `DieselCategoryRepository` and `DieselListingRepository` implement their respective traits
4. All repository methods return correct results