use async_trait::async_trait;
use uuid::Uuid;

use crate::favorites::models::{Favorite, NewFavorite};
use crate::listing::models::Listing;
use crate::shared::errors::AppResult;

#[async_trait]
pub trait FavoriteRepository: Send + Sync {
    async fn create(&self, favorite: NewFavorite) -> AppResult<Favorite>;
    async fn delete(&self, user_id: Uuid, listing_id: Uuid) -> AppResult<bool>;
    async fn find_by_user(
        &self,
        user_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> AppResult<PaginatedFavorites>;
    async fn check(&self, user_id: Uuid, listing_id: Uuid) -> AppResult<bool>;
}

pub struct PaginatedFavorites {
    pub items: Vec<(Favorite, Listing)>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}
