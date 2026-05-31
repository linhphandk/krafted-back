use async_trait::async_trait;
use diesel::pg::Pg;
use diesel::prelude::*;
use uuid::Uuid;

use crate::listing::models::{
    Category, Listing, ListingFilters, ListingSort, NewListing, PaginatedResult, UpdateListing,
};
use crate::listing::ports::{CategoryRepository, ListingRepository};
use crate::schema::{categories, listings};
use crate::shared::db::DbPool;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct DieselCategoryRepository {
    pool: DbPool,
}

impl DieselCategoryRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[derive(Clone)]
pub struct DieselListingRepository {
    pool: DbPool,
}

impl DieselListingRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CategoryRepository for DieselCategoryRepository {
    async fn find_all(&self) -> AppResult<Vec<Category>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        categories::table.load::<Category>(&mut conn).map_err(|e| {
            tracing::error!("Database error: {:?}", e);
            AppError::Internal
        })
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Category>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        categories::table
            .find(id)
            .first::<Category>(&mut conn)
            .optional()
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })
    }

    async fn find_by_kind(&self, kind: &str) -> AppResult<Vec<Category>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        categories::table
            .filter(categories::kind.eq(kind))
            .load::<Category>(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })
    }
}

#[async_trait]
impl ListingRepository for DieselListingRepository {
    async fn create(&self, listing: NewListing) -> AppResult<Listing> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::insert_into(listings::table)
            .values(&listing)
            .get_result::<Listing>(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Listing>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        listings::table
            .find(id)
            .first::<Listing>(&mut conn)
            .optional()
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })
    }

    async fn find_all(
        &self,
        filters: ListingFilters,
        page: i64,
        per_page: i64,
    ) -> AppResult<PaginatedResult<Listing>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;

        let category_ids = if let Some(kind) = &filters.kind {
            let ids: Vec<Uuid> = categories::table
                .filter(categories::kind.eq(kind.to_string()))
                .select(categories::id)
                .load(&mut conn)
                .map_err(|e| {
                    tracing::error!("Database error: {:?}", e);
                    AppError::Internal
                })?;
            if ids.is_empty() {
                None
            } else {
                Some(ids)
            }
        } else {
            None
        };

        let search_pattern = filters.search.as_ref().map(|s| format!("%{}%", s));

        let mut data_query = listings::table.into_boxed::<Pg>();
        let mut count_query = listings::table.into_boxed::<Pg>();

        if let Some(status) = &filters.status {
            data_query = data_query.filter(listings::status.eq(status));
            count_query = count_query.filter(listings::status.eq(status));
        }

        if let Some(category_id) = filters.category_id {
            data_query = data_query.filter(listings::category_id.eq(category_id));
            count_query = count_query.filter(listings::category_id.eq(category_id));
        }

        if let Some(ref ids) = category_ids {
            data_query = data_query.filter(listings::category_id.eq_any(ids));
            count_query = count_query.filter(listings::category_id.eq_any(ids));
        }

        if let Some(ref pattern) = search_pattern {
            data_query = data_query.filter(listings::title.ilike(pattern));
            count_query = count_query.filter(listings::title.ilike(pattern));
        }

        let total = count_query
            .count()
            .get_result::<i64>(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })?;

        match filters.sort {
            ListingSort::Newest => data_query = data_query.order(listings::created_at.desc()),
            ListingSort::PriceAsc => data_query = data_query.order(listings::price_cents.asc()),
            ListingSort::PriceDesc => data_query = data_query.order(listings::price_cents.desc()),
        }

        let offset = (page - 1).max(0) * per_page;
        data_query = data_query.offset(offset).limit(per_page);

        let items = data_query.load::<Listing>(&mut conn).map_err(|e| {
            tracing::error!("Database error: {:?}", e);
            AppError::Internal
        })?;

        Ok(PaginatedResult {
            items,
            total,
            page,
            per_page,
        })
    }

    async fn find_by_seller(
        &self,
        seller_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> AppResult<PaginatedResult<Listing>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;

        let total = listings::table
            .filter(listings::seller_id.eq(seller_id))
            .count()
            .get_result::<i64>(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })?;

        let offset = (page - 1).max(0) * per_page;
        let items = listings::table
            .filter(listings::seller_id.eq(seller_id))
            .order(listings::created_at.desc())
            .offset(offset)
            .limit(per_page)
            .load::<Listing>(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })?;

        Ok(PaginatedResult {
            items,
            total,
            page,
            per_page,
        })
    }

    async fn update(&self, id: Uuid, data: UpdateListing) -> AppResult<Listing> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::update(listings::table.find(id))
            .set(&data)
            .get_result::<Listing>(&mut conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    AppError::NotFound("Listing not found".to_string())
                }
                _ => {
                    tracing::error!("Database error: {:?}", e);
                    AppError::Internal
                }
            })
    }

    async fn delete(&self, id: Uuid) -> AppResult<()> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        let affected = diesel::delete(listings::table.find(id))
            .execute(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })?;
        if affected == 0 {
            return Err(AppError::NotFound("Listing not found".to_string()));
        }
        Ok(())
    }

    async fn count_by_seller(&self, seller_id: Uuid) -> AppResult<i64> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        listings::table
            .filter(listings::seller_id.eq(seller_id))
            .count()
            .get_result::<i64>(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })
    }
}
