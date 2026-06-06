use async_trait::async_trait;
use uuid::Uuid;

use crate::listing::models::{
    Category, Listing, ListingFilters, ListingImage, NewListing, NewListingImage, PaginatedResult,
    UpdateListing,
};
use crate::shared::errors::AppResult;

#[async_trait]
pub trait CategoryRepository: Send + Sync {
    async fn find_all(&self) -> AppResult<Vec<Category>>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Category>>;
    async fn find_by_kind(&self, kind: &str) -> AppResult<Vec<Category>>;
}

#[async_trait]
pub trait ListingRepository: Send + Sync {
    async fn create(&self, listing: NewListing) -> AppResult<Listing>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Listing>>;
    async fn find_all(
        &self,
        filters: ListingFilters,
        page: i64,
        per_page: i64,
    ) -> AppResult<PaginatedResult<Listing>>;
    async fn find_by_seller(
        &self,
        seller_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> AppResult<PaginatedResult<Listing>>;
    async fn update(&self, id: Uuid, data: UpdateListing) -> AppResult<Listing>;
    async fn delete(&self, id: Uuid) -> AppResult<()>;
    async fn count_by_seller(&self, seller_id: Uuid) -> AppResult<i64>;
}

#[async_trait]
pub trait ListingImageRepository: Send + Sync {
    async fn create(&self, image: NewListingImage) -> AppResult<ListingImage>;
    async fn find_by_listing(&self, listing_id: Uuid) -> AppResult<Vec<ListingImage>>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<ListingImage>>;
    async fn delete(&self, id: Uuid) -> AppResult<()>;
    async fn next_position(&self, listing_id: Uuid) -> AppResult<i32>;
}
