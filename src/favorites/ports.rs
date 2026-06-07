use async_trait::async_trait;
use uuid::Uuid;

use crate::favorites::models::{Favorite, FavoriteWithListing, NewFavorite};
use crate::shared::errors::AppResult;

#[async_trait]
pub trait FavoriteRepository: Send + Sync {
    async fn create(&self, favorite: NewFavorite) -> AppResult<Favorite>;
    async fn find_by_user(
        &self,
        user_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> AppResult<Vec<FavoriteWithListing>>;
    async fn count_by_user(&self, user_id: Uuid) -> AppResult<i64>;
    async fn find_by_user_and_listing(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> AppResult<Option<Favorite>>;
    async fn delete(&self, id: Uuid) -> AppResult<()>;
}
