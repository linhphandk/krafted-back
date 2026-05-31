# M12 — Listing Domain: Controller Layer (HTTP API)

**Deliverable: All listing and category HTTP endpoints, router wired, OpenAPI docs updated**

**Depends on: M11**

---

## Task 1 — Define query parameter structs

**Files**: `src/listing/controller.rs`

- `ListListingsQuery` (derive `Deserialize`, `IntoParams`):
  - `status: Option<String>` — filter by listing status
  - `category_id: Option<String>` — filter by category (UUID string)
  - `kind: Option<String>` — filter by category kind ("craft" or "supply")
  - `search: Option<String>` — search by title
  - `sort: Option<String>` — sort order ("newest", "price_asc", "price_desc")
  - `page: Option<i64>` — default 1
  - `per_page: Option<i64>` — default 20

- `ListCategoriesQuery` (derive `Deserialize`, `IntoParams`):
  - `kind: Option<String>` — filter by kind

- `CreateListingImageRequest` — NOT needed yet (deferred to image upload milestone)

---

## Task 2 — Create controller handler functions

**Files**: `src/listing/controller.rs`

Add `#[utoipa::path]` annotations and `ToSchema` derives on all DTOs.

Public endpoints (no auth required):

1. `GET /api/listings` — `list_listings`
   - Accept `ListListingsQuery` as `Query` params
   - Map query params to `ListingFilters`
   - Call `listing_service.list_listings(filters, page, per_page)`
   - Return `Json<PaginatedResponse<ListingResponse>>`

2. `GET /api/listings/:id` — `get_listing`
   - Accept `Path(id: Uuid)`
   - Call `listing_service.get_listing(id)`
   - Return `Json<ListingResponse>`

3. `GET /api/categories` — `list_categories`
   - Call `category_service.list_categories()`
   - Return `Json<Vec<Category>>`

4. `GET /api/categories?kind=craft|supply` — `list_categories_by_kind`
   - Accept `ListCategoriesQuery` as `Query` params
   - If `kind` provided, call `category_service.list_categories_by_kind(kind)`
   - Otherwise call `category_service.list_categories()`

Protected endpoints (auth required — use `State` for now, ownership check in service):

5. `POST /api/listings` — `create_listing`
   - Accept `State(state)` and `Json(req: CreateListingRequest)`
   - Extract seller_id from auth (TODO: M13 will add `AuthUser` extractor)
   - Call `listing_service.create_listing(seller_id, req)`
   - Return `(StatusCode::CREATED, Json<ListingResponse>)`

6. `PATCH /api/listings/:id` — `update_listing`
   - Accept `Path(id: Uuid)`, `State`, `Json(req: UpdateListingRequest)`
   - Extract seller_id from auth
   - Call `listing_service.update_listing(id, seller_id, req)`
   - Return `Json<ListingResponse>`

7. `DELETE /api/listings/:id` — `delete_listing`
   - Accept `Path(id: Uuid)`, `State`
   - Extract seller_id from auth
   - Call `listing_service.delete_listing(id, seller_id)`
   - Return `StatusCode::NO_CONTENT`

8. `POST /api/listings/:id/publish` — `publish_listing`
   - Accept `Path(id: Uuid)`, `State`
   - Extract seller_id from auth
   - Call `listing_service.publish_listing(id, seller_id)`
   - Return `Json<ListingResponse>`

9. `POST /api/listings/:id/pause` — `pause_listing`
   - Accept `Path(id: Uuid)`, `State`
   - Extract seller_id from auth
   - Call `listing_service.pause_listing(id, seller_id)`
   - Return `Json<ListingResponse>`

10. `GET /api/listings/mine` — `seller_listings`
    - Accept `State`, `Query(page, per_page)`
    - Extract seller_id from auth
    - Call `listing_service.list_my_listings(seller_id, page, per_page)`
    - Return `Json<PaginatedResponse<ListingResponse>>`

**For Task 2, use placeholder seller_id extraction.** M13 will add proper `AuthUser` extractor. For now, protected endpoints can accept a `seller_id` query param or header, or comment out auth-dependent logic with a `// TODO: M13` note. The important thing is the route definitions and service wiring work.

---

## Task 3 — Create `listing_router()`

**Files**: `src/listing/mod.rs`

```rust
pub fn listing_router() -> axum::Router<AppState> {
    axum::Router::<AppState>::new()
        .route("/api/listings", axum::routing::get(controller::list_listings).post(controller::create_listing))
        .route("/api/listings/mine", axum::routing::get(controller::seller_listings))
        .route("/api/listings/{id}", axum::routing::get(controller::get_listing).patch(controller::update_listing).delete(controller::delete_listing))
        .route("/api/listings/{id}/publish", axum::routing::post(controller::publish_listing))
        .route("/api/listings/{id}/pause", axum::routing::post(controller::pause_listing))
        .route("/api/categories", axum::routing::get(controller::list_categories))
}
```

Note: `/api/listings/mine` must come before `/api/listings/{id}` to avoid "mine" being captured as an ID.

---

## Task 4 — Update `AppState`

**Files**: `src/state.rs`

Add fields:
- `listing_service: ListingService<DieselListingRepository, DieselCategoryRepository>`
- `category_service: CategoryService<DieselCategoryRepository>`

Initialize in `AppState::new()`:
```rust
let category_repo = DieselCategoryRepository::new(pool.clone());
let listing_repo = DieselListingRepository::new(pool.clone());
let listing_service = ListingService::new(listing_repo, category_repo.clone());
let category_service = CategoryService::new(category_repo);
```

---

## Task 5 — Wire `listing_router()` into main router

**Files**: `src/router.rs`

```rust
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api-docs/openapi.json", get(openapi_json))
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .merge(auth_router())
        .merge(listing_router())
        .with_state(state)
}
```

---

## Task 6 — Update OpenAPI docs

**Files**: `src/api_doc.rs`

Add `listing::models::*` to `schemas` and `listing::controller` to `mod_path` in `OpenApi` derive.

Add `ToSchema` derives to all request/response types in `listing/models.rs`.

---

## Task 7 — `cargo fmt && cargo clippy && cargo check`

Fix any warnings. Verify all endpoints compile.

---

## Verification

1. `cargo check` passes
2. `cargo clippy` passes
3. Server starts, routes are registered
4. `GET /api/categories` returns seeded categories
5. `GET /api/listings` returns empty paginated result
6. `GET /api/listings/{id}` returns 404 for non-existent listing
7. OpenAPI spec at `/api-docs/openapi.json` includes listing and category endpoints