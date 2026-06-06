use async_trait::async_trait;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::favorites::models::{Favorite, NewFavorite};
use crate::favorites::ports::{FavoriteRepository, PaginatedFavorites};
use crate::listing::models::Listing;
use crate::schema::{favorites, listings};
use crate::shared::db::DbPool;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct DieselFavoriteRepository {
    pool: DbPool,
}

impl DieselFavoriteRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FavoriteRepository for DieselFavoriteRepository {
    #[instrument(skip(self), fields(user_id = %favorite.user_id))]
    async fn create(&self, favorite: NewFavorite) -> AppResult<Favorite> {
        debug!("create favorite");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::insert_into(favorites::table)
            .values(&favorite)
            .get_result::<Favorite>(&mut conn)
            .map_err(|e| {
                if let diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) =
                    &e
                {
                    AppError::BadRequest("Already favorited".to_string())
                } else {
                    tracing::error!("Database error: {:?}", e);
                    AppError::Internal
                }
            })
    }

    #[instrument(skip(self), fields(user_id = %user_id, listing_id = %listing_id))]
    async fn delete(&self, user_id: Uuid, listing_id: Uuid) -> AppResult<bool> {
        debug!("delete favorite");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        let affected = diesel::delete(
            favorites::table
                .filter(favorites::user_id.eq(user_id))
                .filter(favorites::listing_id.eq(listing_id)),
        )
        .execute(&mut conn)
        .map_err(|e| {
            tracing::error!("Database error: {:?}", e);
            AppError::Internal
        })?;
        Ok(affected > 0)
    }

    #[instrument(skip(self), fields(user_id = %user_id))]
    async fn find_by_user(
        &self,
        user_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> AppResult<PaginatedFavorites> {
        debug!(user_id = %user_id, page = %page, per_page = %per_page, "find favorites by user");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;

        let offset = (page - 1) * per_page;

        let items: Vec<(Favorite, Listing)> = favorites::table
            .inner_join(listings::table.on(favorites::listing_id.eq(listings::id)))
            .filter(favorites::user_id.eq(user_id))
            .order_by(favorites::created_at.desc())
            .offset(offset)
            .limit(per_page)
            .load::<(Favorite, Listing)>(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })?;

        let total: i64 = favorites::table
            .filter(favorites::user_id.eq(user_id))
            .count()
            .get_result(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })?;

        Ok(PaginatedFavorites {
            items,
            total,
            page,
            per_page,
        })
    }

    #[instrument(skip(self), fields(user_id = %user_id, listing_id = %listing_id))]
    async fn check(&self, user_id: Uuid, listing_id: Uuid) -> AppResult<bool> {
        debug!("check favorite");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        let count: i64 = favorites::table
            .filter(favorites::user_id.eq(user_id))
            .filter(favorites::listing_id.eq(listing_id))
            .count()
            .get_result(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })?;
        Ok(count > 0)
    }
}
