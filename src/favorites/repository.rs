use async_trait::async_trait;
use diesel::prelude::*;
use tracing::instrument;
use uuid::Uuid;

use crate::favorites::models::{Favorite, FavoriteWithListing, NewFavorite};
use crate::favorites::ports::FavoriteRepository;
use crate::schema::favorites;
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
    #[instrument(skip(self))]
    async fn create(&self, favorite: NewFavorite) -> AppResult<Favorite> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::insert_into(favorites::table)
            .values(&favorite)
            .get_result::<Favorite>(&mut conn)
            .map_err(|e| match e {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                ) => {
                    tracing::warn!("Duplicate favorite: {:?}", e);
                    AppError::BadRequest("Already favorited".to_string())
                }
                _ => {
                    tracing::error!("Database error: {:?}", e);
                    AppError::Internal
                }
            })
    }

    #[instrument(skip(self))]
    async fn find_by_user(
        &self,
        user_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> AppResult<Vec<FavoriteWithListing>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        let offset = (page - 1) * per_page;
        diesel::sql_query(
            r#"
            SELECT
                f.id, f.user_id, f.listing_id, f.created_at,
                l.title, l.price_cents, l.status,
                (SELECT li.thumbnail_url FROM listing_images li WHERE li.listing_id = l.id ORDER BY li.position LIMIT 1) AS thumbnail_url,
                u.id AS seller_id,
                u.name AS seller_name
            FROM favorites f
            INNER JOIN listings l ON l.id = f.listing_id
            INNER JOIN users u ON u.id = l.seller_id
            WHERE f.user_id = $1
            ORDER BY f.created_at DESC
            OFFSET $2 LIMIT $3
            "#,
        )
        .bind::<diesel::sql_types::Uuid, _>(user_id)
        .bind::<diesel::sql_types::BigInt, _>(offset)
        .bind::<diesel::sql_types::BigInt, _>(per_page)
        .load::<FavoriteWithListing>(&mut conn)
        .map_err(|e| {
            tracing::error!("Database error: {:?}", e);
            AppError::Internal
        })
    }

    #[instrument(skip(self))]
    async fn count_by_user(&self, user_id: Uuid) -> AppResult<i64> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        favorites::table
            .filter(favorites::user_id.eq(user_id))
            .count()
            .get_result::<i64>(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })
    }

    #[instrument(skip(self))]
    async fn find_by_user_and_listing(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> AppResult<Option<Favorite>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        match favorites::table
            .filter(favorites::user_id.eq(user_id))
            .filter(favorites::listing_id.eq(listing_id))
            .first::<Favorite>(&mut conn)
        {
            Ok(fav) => Ok(Some(fav)),
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(e) => {
                tracing::error!("Database error: {:?}", e);
                Err(AppError::Internal)
            }
        }
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: Uuid) -> AppResult<()> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        let affected = diesel::delete(favorites::table.find(id))
            .execute(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })?;
        if affected == 0 {
            return Err(AppError::NotFound("Favorite not found".to_string()));
        }
        Ok(())
    }
}
