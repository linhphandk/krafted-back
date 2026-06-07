use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::favorites::models::{FavoriteResponse, NewFavorite};
use crate::favorites::ports::FavoriteRepository;
use crate::listing::models::PaginatedResult;
use crate::listing::ports::ListingRepository;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct FavoritesService<R, L> {
    repo: R,
    listing_repo: L,
}

impl<R: FavoriteRepository, L: ListingRepository> FavoritesService<R, L> {
    pub fn new(repo: R, listing_repo: L) -> Self {
        Self { repo, listing_repo }
    }

    #[instrument(skip(self), fields(user_id = %user_id, listing_id = %listing_id))]
    pub async fn add_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> AppResult<FavoriteResponse> {
        let listing = self
            .listing_repo
            .find_by_id(listing_id)
            .await?
            .ok_or_else(|| {
                warn!(listing_id = %listing_id, "add_favorite: listing not found");
                AppError::NotFound("Listing not found".to_string())
            })?;

        if listing.status != "active" {
            warn!(listing_id = %listing_id, status = %listing.status, "add_favorite: listing not active");
            return Err(AppError::BadRequest(
                "Cannot favorite a listing that is not active".to_string(),
            ));
        }

        let existing = self
            .repo
            .find_by_user_and_listing(user_id, listing_id)
            .await?;
        if let Some(fav) = existing {
            info!(user_id = %user_id, listing_id = %listing_id, "add_favorite: already exists");
            return Ok(FavoriteResponse::from_favorite(&fav));
        }

        let fav = self
            .repo
            .create(NewFavorite {
                user_id,
                listing_id,
            })
            .await?;

        info!(favorite_id = %fav.id, "favorite added");
        Ok(FavoriteResponse::from_favorite(&fav))
    }

    #[instrument(skip(self), fields(user_id = %user_id, listing_id = %listing_id))]
    pub async fn remove_favorite(&self, user_id: Uuid, listing_id: Uuid) -> AppResult<()> {
        let fav = self
            .repo
            .find_by_user_and_listing(user_id, listing_id)
            .await?
            .ok_or_else(|| {
                warn!(
                    user_id = %user_id,
                    listing_id = %listing_id,
                    "remove_favorite: not found"
                );
                AppError::NotFound("Favorite not found".to_string())
            })?;

        self.repo.delete(fav.id).await?;
        info!(favorite_id = %fav.id, "favorite removed");
        Ok(())
    }

    #[instrument(skip(self), fields(user_id = %user_id, page, per_page))]
    pub async fn list_favorites(
        &self,
        user_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> AppResult<PaginatedResult<FavoriteResponse>> {
        let items = self.repo.find_by_user(user_id, page, per_page).await?;
        let total = self.repo.count_by_user(user_id).await?;

        let favorites: Vec<FavoriteResponse> =
            items.iter().map(FavoriteResponse::from_favorite).collect();

        Ok(PaginatedResult {
            items: favorites,
            total,
            page,
            per_page,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDateTime;
    use mockall::mock;

    use crate::favorites::models::Favorite;
    use crate::listing::models::{Listing, ListingFilters, NewListing, UpdateListing};

    mock! {
        pub FavRepoMock {}

        #[async_trait]
        impl FavoriteRepository for FavRepoMock {
            async fn create(&self, favorite: NewFavorite) -> AppResult<Favorite>;
            async fn find_by_user(&self, user_id: Uuid, page: i64, per_page: i64) -> AppResult<Vec<Favorite>>;
            async fn count_by_user(&self, user_id: Uuid) -> AppResult<i64>;
            async fn find_by_user_and_listing(&self, user_id: Uuid, listing_id: Uuid) -> AppResult<Option<Favorite>>;
            async fn delete(&self, id: Uuid) -> AppResult<()>;
        }
    }

    mock! {
        pub ListingRepoForFavMock {}

        #[async_trait]
        impl ListingRepository for ListingRepoForFavMock {
            async fn create(&self, listing: NewListing) -> AppResult<Listing>;
            async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Listing>>;
            async fn find_all(&self, filters: ListingFilters, page: i64, per_page: i64) -> AppResult<PaginatedResult<Listing>>;
            async fn find_by_seller(&self, seller_id: Uuid, page: i64, per_page: i64) -> AppResult<PaginatedResult<Listing>>;
            async fn update(&self, id: Uuid, data: UpdateListing) -> AppResult<Listing>;
            async fn delete(&self, id: Uuid) -> AppResult<()>;
            async fn count_by_seller(&self, seller_id: Uuid) -> AppResult<i64>;
        }
    }

    fn dummy_listing(id: Uuid, seller_id: Uuid, status: &str) -> Listing {
        Listing {
            id,
            seller_id,
            title: "Test".to_string(),
            description: "".to_string(),
            price_cents: 1000,
            category_id: Uuid::new_v4(),
            status: status.to_string(),
            condition: "handmade".to_string(),
            quantity: 1,
            created_at: NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
            updated_at: NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
        }
    }

    fn dummy_favorite(id: Uuid, user_id: Uuid, listing_id: Uuid) -> Favorite {
        Favorite {
            id,
            user_id,
            listing_id,
            created_at: NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
        }
    }

    fn service_with_mocks(
        repo: MockFavRepoMock,
        listing_repo: MockListingRepoForFavMock,
    ) -> FavoritesService<MockFavRepoMock, MockListingRepoForFavMock> {
        FavoritesService::new(repo, listing_repo)
    }

    #[tokio::test]
    async fn test_add_favorite_listing_not_found() {
        let mut fav_repo = MockFavRepoMock::new();
        let mut listing_repo = MockListingRepoForFavMock::new();

        listing_repo.expect_find_by_id().return_once(|_| Ok(None));

        let service = service_with_mocks(fav_repo, listing_repo);
        let result = service.add_favorite(Uuid::new_v4(), Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_add_favorite_listing_not_active() {
        let mut fav_repo = MockFavRepoMock::new();
        let mut listing_repo = MockListingRepoForFavMock::new();
        let listing_id = Uuid::new_v4();

        listing_repo
            .expect_find_by_id()
            .withf(move |id| *id == listing_id)
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, Uuid::new_v4(), "draft"))));

        let service = service_with_mocks(fav_repo, listing_repo);
        let result = service.add_favorite(Uuid::new_v4(), listing_id).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_add_favorite_already_exists() {
        let mut fav_repo = MockFavRepoMock::new();
        let mut listing_repo = MockListingRepoForFavMock::new();
        let user_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let fav_id = Uuid::new_v4();

        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, Uuid::new_v4(), "active"))));

        let fav = dummy_favorite(fav_id, user_id, listing_id);
        fav_repo
            .expect_find_by_user_and_listing()
            .withf(move |u, l| *u == user_id && *l == listing_id)
            .return_once(|_, _| Ok(Some(fav)));

        let service = service_with_mocks(fav_repo, listing_repo);
        let result = service.add_favorite(user_id, listing_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().listing_id, listing_id.to_string());
    }

    #[tokio::test]
    async fn test_add_favorite_success() {
        let mut fav_repo = MockFavRepoMock::new();
        let mut listing_repo = MockListingRepoForFavMock::new();
        let user_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let fav_id = Uuid::new_v4();

        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, Uuid::new_v4(), "active"))));

        fav_repo
            .expect_find_by_user_and_listing()
            .return_once(|_, _| Ok(None));

        let new_fav = dummy_favorite(fav_id, user_id, listing_id);
        fav_repo
            .expect_create()
            .withf(move |nf: &NewFavorite| nf.user_id == user_id && nf.listing_id == listing_id)
            .return_once(|_| Ok(new_fav));

        let service = service_with_mocks(fav_repo, listing_repo);
        let result = service.add_favorite(user_id, listing_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().listing_id, listing_id.to_string());
    }

    #[tokio::test]
    async fn test_remove_favorite_not_found() {
        let mut fav_repo = MockFavRepoMock::new();
        let mut listing_repo = MockListingRepoForFavMock::new();

        fav_repo
            .expect_find_by_user_and_listing()
            .return_once(|_, _| Ok(None));

        let service = service_with_mocks(fav_repo, listing_repo);
        let result = service
            .remove_favorite(Uuid::new_v4(), Uuid::new_v4())
            .await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_remove_favorite_success() {
        let mut fav_repo = MockFavRepoMock::new();
        let mut listing_repo = MockListingRepoForFavMock::new();
        let user_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let fav_id = Uuid::new_v4();

        let fav = dummy_favorite(fav_id, user_id, listing_id);
        fav_repo
            .expect_find_by_user_and_listing()
            .return_once(|_, _| Ok(Some(fav)));

        fav_repo
            .expect_delete()
            .withf(move |id| *id == fav_id)
            .return_once(|_| Ok(()));

        let service = service_with_mocks(fav_repo, listing_repo);
        let result = service.remove_favorite(user_id, listing_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_favorites_empty() {
        let mut fav_repo = MockFavRepoMock::new();
        let mut listing_repo = MockListingRepoForFavMock::new();
        let user_id = Uuid::new_v4();

        fav_repo
            .expect_find_by_user()
            .return_once(|_, _, _| Ok(vec![]));
        fav_repo.expect_count_by_user().return_once(|_| Ok(0));

        let service = service_with_mocks(fav_repo, listing_repo);
        let result = service.list_favorites(user_id, 1, 20).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().items.len(), 0);
    }

    #[tokio::test]
    async fn test_list_favorites_returns_all() {
        let mut fav_repo = MockFavRepoMock::new();
        let mut listing_repo = MockListingRepoForFavMock::new();
        let user_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let fav_id = Uuid::new_v4();

        let fav = dummy_favorite(fav_id, user_id, listing_id);
        fav_repo
            .expect_find_by_user()
            .return_once(move |_, _, _| Ok(vec![fav]));
        fav_repo.expect_count_by_user().return_once(|_| Ok(1));

        let service = service_with_mocks(fav_repo, listing_repo);
        let result = service.list_favorites(user_id, 1, 20).await;
        assert!(result.is_ok());
        let paginated = result.unwrap();
        assert_eq!(paginated.items.len(), 1);
        assert_eq!(paginated.items[0].listing_id, listing_id.to_string());
        assert_eq!(paginated.total, 1);
    }
}
