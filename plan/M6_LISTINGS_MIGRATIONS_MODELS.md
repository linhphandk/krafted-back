# M6 — Listing Domain: Migrations + Models + Schema

**Deliverable: Diesel migrations for categories/listings, domain models, schema generated, new error variants**

**Depends on: M1-M5 (existing auth/user/session foundation)**

---

## Task 1 — Create `categories` migration

**Files**: `migrations/<timestamp>_create_categories/up.sql`, `migrations/<timestamp>_create_categories/down.sql`

Create `categories` table:

```sql
CREATE TABLE categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    slug VARCHAR(100) NOT NULL UNIQUE,
    kind VARCHAR(20) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_categories_kind ON categories(kind);
```

`down.sql`: `DROP TABLE categories;`

No `parent_id` yet — flat list for MVP.

---

## Task 2 — Create `listings` migration

**Files**: `migrations/<timestamp>_create_listings/up.sql`, `migrations/<timestamp>_create_listings/down.sql`

```sql
CREATE TABLE listings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    seller_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    price_cents INTEGER NOT NULL,
    category_id UUID NOT NULL REFERENCES categories(id),
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    condition VARCHAR(20) NOT NULL DEFAULT 'handmade',
    quantity INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_listings_seller_id ON listings(seller_id);
CREATE INDEX idx_listings_category_id ON listings(category_id);
CREATE INDEX idx_listings_status ON listings(status);
CREATE INDEX idx_listings_created_at ON listings(created_at DESC);
```

`down.sql`: `DROP TABLE listings;`

No `currency` column (hardcode USD). No `listing_images` table (deferred). No `shipping_price_cents`/`shipping_description` (deferred).

---

## Task 3 — Create `seed_categories` migration

**Files**: `migrations/<timestamp>_seed_categories/up.sql`, `migrations/<timestamp>_seed_categories/down.sql`

Insert 15 categories:

```sql
INSERT INTO categories (name, slug, kind) VALUES
  ('Pottery & Ceramics', 'pottery-ceramics', 'craft'),
  ('Jewelry', 'jewelry', 'craft'),
  ('Woodworking', 'woodworking', 'craft'),
  ('Knitting & Crochet', 'knitting-crochet', 'craft'),
  ('Painting & Art', 'painting-art', 'craft'),
  ('Leathercraft', 'leathercraft', 'craft'),
  ('Candles & Soap', 'candles-soap', 'craft'),
  ('Paper Crafts', 'paper-crafts', 'craft'),
  ('Yarn & Fiber', 'yarn-fiber', 'supply'),
  ('Fabric & Textiles', 'fabric-textiles', 'supply'),
  ('Beads & Jewelry Supplies', 'beads-jewelry-supplies', 'supply'),
  ('Paint & Brushes', 'paint-brushes', 'supply'),
  ('Clay & Ceramics Supplies', 'clay-ceramics-supplies', 'supply'),
  ('Tools & Equipment', 'tools-equipment', 'supply'),
  ('Patterns & Templates', 'patterns-templates', 'supply');
```

`down.sql`: `DELETE FROM categories WHERE slug IN (...);`

---

## Task 4 — Run `diesel print-schema` and verify

Run `diesel print-schema > src/schema.rs` to regenerate. Verify the generated `diesel::table!` macros include `categories` and `listings`. Run `cargo check`.

---

## Task 5 — Add error variants to `AppError`

**Files**: `src/shared/errors.rs`

Add variants:

- `NotFound(String)` → 404
- `Unauthorized` → 401
- `Forbidden(String)` → 403

Update `IntoResponse` impl to handle these with correct status codes.

---

## Task 6 — Create `listing` module and models

**Files**: `src/listing/mod.rs`, `src/listing/models.rs`

`src/listing/mod.rs`:
```rust
pub mod models;
```

`src/listing/models.rs` — define:

- `Category` — `Queryable, Selectable, Serialize, Clone, Debug` with diesel derive
- `NewCategory` — `Insertable`
- `CategoryKind` — enum (`Craft`, `Supply`), with `Display`, `FromStr`, `Deserialize` for query param parsing. Map to/from `VARCHAR` in diesel.
- `Listing` — `Queryable, Selectable, Serialize, Clone, Debug`
- `NewListing` — `Insertable` (seller_id, title, description, price_cents, category_id, status, condition, quantity)
- `UpdateListing` — `AsChangeset` with all fields `Option<>`, `#[diesel(treat_none_as_null = false)]`
- `ListingStatus` — enum (`Draft`, `Active`, `Paused`, `Closed`), `Display`, `FromStr`, `Deserialize`
- `ListingCondition` — enum (`Handmade`, `New`, `Vintage`, `Refurbished`), `Display`, `FromStr`, `Deserialize`
- `ListingFilters` — status, category_id, kind (Option<CategoryKind>), search (Option<String>), sort (ListingSort)
- `ListingSort` — enum (`Newest`, `PriceAsc`, `PriceDesc`)
- `PaginatedResult<T>` — items: Vec<T>, total: i64, page: i64, per_page: i64

---

## Task 7 — Update `src/lib.rs`

**Files**: `src/lib.rs`

Add `pub mod listing;`

---

## Task 8 — `cargo check` — all models compile

Verify everything compiles cleanly. Diesel derives should match the schema.

---

## Verification

1. `cargo check` passes
2. `sql_schema.rs` contains `categories` and `listings` table macros
3. `AppError::NotFound`, `AppError::Unauthorized`, `AppError::Forbidden` compile and return correct HTTP status codes
4. `ListingStatus`, `ListingCondition`, `CategoryKind` have Display/FromStr/Deserialize