# M10 — Listing Image Upload to S3

**Deliverable: Reusable `ImageStorage` port + `S3ImageStorage` adapter in shared/, new `listing_image` domain module with multipart upload endpoint that stores images to S3**

**Depends on: M9 (listing controllers complete)**

---

## Principle: Reusable Storage Abstraction

The `ImageStorage` port trait and `S3ImageStorage` adapter live in `src/shared/image_storage.rs` so any domain can import them. The `listing_image` module is one consumer — it depends on `ImageStorage` via its constructor, receiving the storage adapter and bucket name at instantiation time.

---

## Task 1 — Add dependencies (~10loc Cargo.toml)

Add to `[dependencies]`:
```toml
aws-sdk-s3 = "1"
aws-config = "1"
bytes = "1"
```

---

## Task 2 — `ImageStorage` port + `S3ImageStorage` adapter (~80loc)

**File**: `src/shared/image_storage.rs`

Port trait:
```rust
#[async_trait]
pub trait ImageStorage: Send + Sync {
    async fn upload(&self, bucket: &str, key: &str, data: Vec<u8>, content_type: &str) -> AppResult<String>;
    async fn delete(&self, bucket: &str, key: &str) -> AppResult<()>;
}
```

`S3ImageStorage` struct:
- Holds only `client: aws_sdk_s3::Client`
- `new()` loads config from env via `aws_config::load_from_env()`
- `upload()` calls `client.put_object()` with bucket/key/data/content-type, returns the S3 URL
- `delete()` calls `client.delete_object()` with bucket/key
- No bucket or path prefix stored on the struct — caller specifies per-call

---

## Task 3 — Migration: `listing_images` table (~20loc SQL)

**Files**: `migrations/20260602000000_create_listing_images/up.sql`, `down.sql`

```sql
CREATE TABLE listing_images (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    s3_key VARCHAR(512) NOT NULL,
    url VARCHAR(1024) NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_listing_images_listing_id ON listing_images(listing_id);
```

---

## Task 4 — Config: AWS env vars (~5loc)

**File**: `src/shared/config.rs`

No new config fields needed — bucket is passed at the call site. SDK reads `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION` from env automatically.

---

## Task 5 — `listing_image` domain module

### 5a. Module entry + router (`src/listing_image/mod.rs`)

```rust
pub mod controller;
pub mod models;
pub mod ports;
pub mod repository;
pub mod service;

pub fn listing_image_router() -> axum::Router<crate::state::AppState> {
    axum::Router::new()
        .route("/api/listings/{id}/images",
            axum::routing::post(controller::upload_images)
                .get(controller::list_images))
        .route("/api/listings/{id}/images/{image_id}",
            axum::routing::delete(controller::delete_image))
}
```

### 5b. Models (`src/listing_image/models.rs`) ~50loc

- `ListingImage` — Queryable, Selectable, Serialize, Clone
- `NewListingImage` — Insertable
- `ImageResponse` — Serialize, ToSchema (id, listing_id, url, position, created_at)

### 5c. Ports (`src/listing_image/ports.rs`) ~15loc

```rust
#[async_trait]
pub trait ListingImageRepository: Send + Sync {
    async fn create(&self, image: NewListingImage) -> AppResult<ListingImage>;
    async fn find_by_listing(&self, listing_id: Uuid) -> AppResult<Vec<ListingImage>>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<ListingImage>>;
    async fn delete(&self, id: Uuid) -> AppResult<()>;
    async fn next_position(&self, listing_id: Uuid) -> AppResult<i32>;
}
```

### 5d. Service (`src/listing_image/service.rs`) ~100loc

`ListingImageService<L: ListingImageRepository, S: ImageStorage>`:
- `upload_images(listing_id, seller_id, files)` — verifies listing ownership, uploads each file to S3 via `ImageStorage`, saves records via `ListingImageRepository`, returns `Vec<ImageResponse>`
- `list_images(listing_id)` — returns all images for a listing (public)
- `delete_image(listing_id, image_id, seller_id)` — verifies listing ownership, deletes from S3 + DB

S3 key format: `listings/{listing_id}/{uuid}-{sanitized_filename}`

### 5e. Repository (`src/listing_image/repository.rs`) ~60loc

`DieselListingImageRepository` — implements `ListingImageRepository`:
- `create` — INSERT
- `find_by_listing` — SELECT WHERE listing_id = $1 ORDER BY position ASC
- `find_by_id` — SELECT WHERE id = $1
- `delete` — DELETE WHERE id = $1
- `next_position` — SELECT COALESCE(MAX(position), 0) + 1

### 5f. Controller (`src/listing_image/controller.rs`) ~120loc

Three handlers:

1. **`POST /api/listings/{id}/images`** — `upload_images`
   - Accepts `Multipart` form data
   - `x-seller-id` header for auth (same pattern as listing controller)
   - Validates each file: max 10MB, content-type starts with `image/`
   - Calls service for each valid file
   - Returns `201` + `Vec<ImageResponse>`

2. **`GET /api/listings/{id}/images`** — `list_images`
   - Public endpoint (no auth)
   - Returns `200` + `Vec<ImageResponse>`

3. **`DELETE /api/listings/{id}/images/{image_id}`** — `delete_image`
   - `x-seller-id` header for auth + ownership via service
   - Returns `204 No Content`

### 5g. Unit tests (`src/listing_image/service.rs` `#[cfg(test)]`) ~150loc

Mock `ListingImageRepository` and `ImageStorage` with mockall:
- `test_upload_single_image`
- `test_upload_multiple_images`
- `test_upload_nonexistent_listing` — returns NotFound
- `test_upload_not_owner` — returns Forbidden
- `test_list_images`
- `test_delete_image`
- `test_delete_not_owner` — returns Forbidden

---

## Task 6 — Schema update

**File**: `src/schema.rs`

Run `diesel print-schema` to regenerate, or manually add the `listing_images` table macro.

---

## Task 7 — Wire into AppState

**File**: `src/state.rs`

Add fields:
- `listing_image_service: ListingImageService<DieselListingImageRepository, S3ImageStorage>`
- `image_storage: S3ImageStorage`

Initialize in `AppState::new()`:
```rust
let listing_image_repo = DieselListingImageRepository::new(pool.clone());
let image_storage = S3ImageStorage::new().await;
let listing_image_service = ListingImageService::new(
    listing_image_repo,
    image_storage.clone(),
    listing_service,
    std::env::var("AWS_S3_BUCKET").expect("AWS_S3_BUCKET must be set"),
);
```

---

## Task 8 — Register router

**File**: `src/router.rs`

Add `.merge(crate::listing_image::listing_image_router())` to `create_router()`.

**File**: `src/lib.rs`

Add `pub mod listing_image;`

---

## Task 9 — Update OpenAPI docs

**File**: `src/api_doc.rs`

Add image-related paths and schemas.

---

## Task 10 — Update .env.example

**File**: `.env.example`

```
AWS_S3_BUCKET=krafted-images
AWS_REGION=us-east-1
# AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY set via env
```

---

## Task 11 — Integration tests

**Files**: `tests/listing_image_repository.rs`

Use testcontainers:
- Create listing_image records
- Find by listing
- Delete
- Next position increments

## Verification

1. `cargo check` passes
2. `cargo clippy` passes
3. `cargo test -- --test-threads=1` — all tests pass
4. `POST /api/listings/{id}/images` with multipart uploads images to S3 and creates records in DB
5. `GET /api/listings/{id}/images` returns image list (public)
6. `DELETE /api/listings/{id}/images/{image_id}` removes from S3 + DB, ownership enforced
7. `listings/{listing_id}/{uuid}-{filename}` S3 key format
8. Max 10MB file size enforced at controller
9. Only image content-types accepted
10. `ImageStorage` trait imported by any domain for future use (avatars, category icons, etc.)
