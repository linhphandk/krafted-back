use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::listing::models::{
    Category, CreateListingRequest, Listing, ListingFilters, NewListing, PaginatedResult,
    UpdateListing, UpdateListingRequest,
};
use crate::listing::ports::{CategoryRepository, ListingRepository};
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct ListingService<L: ListingRepository, C: CategoryRepository> {
    listing_repo: L,
    category_repo: C,
}

impl<L: ListingRepository, C: CategoryRepository> ListingService<L, C> {
    pub fn new(listing_repo: L, category_repo: C) -> Self {
        Self {
            listing_repo,
            category_repo,
        }
    }

    #[instrument(skip(self, req), fields(seller_id = %seller_id, title = %req.title))]
    pub async fn create_listing(
        &self,
        seller_id: Uuid,
        req: CreateListingRequest,
    ) -> AppResult<Listing> {
        if req.title.trim().is_empty() {
            warn!("create_listing rejected: empty title");
            return Err(AppError::BadRequest("Title cannot be empty".to_string()));
        }
        if req.price_cents <= 0 {
            warn!(
                price_cents = req.price_cents,
                "create_listing rejected: non-positive price"
            );
            return Err(AppError::BadRequest(
                "Price must be greater than 0".to_string(),
            ));
        }
        if self
            .category_repo
            .find_by_id(req.category_id)
            .await?
            .is_none()
        {
            warn!(category_id = %req.category_id, "create_listing rejected: category not found");
            return Err(AppError::BadRequest("Category not found".to_string()));
        }

        let new_listing = NewListing {
            seller_id,
            title: req.title,
            description: req.description,
            price_cents: req.price_cents,
            category_id: req.category_id,
            status: "draft".to_string(),
            condition: req.condition.to_string(),
            quantity: req.quantity.unwrap_or(1),
        };

        let listing = self.listing_repo.create(new_listing).await;
        if let Ok(ref l) = listing {
            info!(listing_id = %l.id, "listing created");
        }
        listing
    }

    #[instrument(skip(self), fields(listing_id = %id))]
    pub async fn get_listing(&self, id: Uuid) -> AppResult<Listing> {
        self.listing_repo.find_by_id(id).await?.ok_or_else(|| {
            warn!(listing_id = %id, "get_listing: not found");
            AppError::NotFound("Listing not found".to_string())
        })
    }

    #[instrument(skip(self), fields(page, per_page))]
    pub async fn list_listings(
        &self,
        mut filters: ListingFilters,
        page: i64,
        per_page: i64,
    ) -> AppResult<PaginatedResult<Listing>> {
        debug!(?filters, page, per_page, "list_listings called");
        if filters.status.is_none() {
            filters.status = Some("active".to_string());
        }
        let result = self.listing_repo.find_all(filters, page, per_page).await;
        if let Ok(ref r) = result {
            debug!(
                total = r.total,
                returned = r.items.len(),
                "list_listings result"
            );
        }
        result
    }

    #[instrument(skip(self), fields(seller_id = %seller_id, page, per_page))]
    pub async fn list_my_listings(
        &self,
        seller_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> AppResult<PaginatedResult<Listing>> {
        let result = self
            .listing_repo
            .find_by_seller(seller_id, page, per_page)
            .await;
        if let Ok(ref r) = result {
            debug!(total = r.total, "seller listings fetched");
        }
        result
    }

    #[instrument(skip(self, req), fields(listing_id = %id, seller_id = %seller_id))]
    pub async fn update_listing(
        &self,
        id: Uuid,
        seller_id: Uuid,
        req: UpdateListingRequest,
    ) -> AppResult<Listing> {
        let listing = self.get_listing(id).await?;
        if listing.seller_id != seller_id {
            warn!(listing_id = %id, owner = %listing.seller_id, caller = %seller_id, "update_listing forbidden: not owner");
            return Err(AppError::Forbidden(
                "You do not own this listing".to_string(),
            ));
        }

        if let Some(category_id) = req.category_id {
            if self.category_repo.find_by_id(category_id).await?.is_none() {
                warn!(category_id = %category_id, "update_listing rejected: category not found");
                return Err(AppError::BadRequest("Category not found".to_string()));
            }
        }

        let data = UpdateListing {
            title: req.title,
            description: req.description,
            price_cents: req.price_cents,
            category_id: req.category_id,
            status: req.status.map(|s| s.to_string()),
            condition: req.condition.map(|c| c.to_string()),
            quantity: req.quantity,
        };

        let result = self.listing_repo.update(id, data).await;
        if result.is_ok() {
            info!(listing_id = %id, "listing updated");
        }
        result
    }

    #[instrument(skip(self), fields(listing_id = %id, seller_id = %seller_id))]
    pub async fn delete_listing(&self, id: Uuid, seller_id: Uuid) -> AppResult<()> {
        let listing = self.get_listing(id).await?;
        if listing.seller_id != seller_id {
            warn!(listing_id = %id, owner = %listing.seller_id, caller = %seller_id, "delete_listing forbidden: not owner");
            return Err(AppError::Forbidden(
                "You do not own this listing".to_string(),
            ));
        }
        let result = self.listing_repo.delete(id).await;
        if result.is_ok() {
            info!(listing_id = %id, "listing deleted");
        }
        result
    }

    #[instrument(skip(self), fields(listing_id = %id, seller_id = %seller_id))]
    pub async fn publish_listing(&self, id: Uuid, seller_id: Uuid) -> AppResult<Listing> {
        let listing = self.get_listing(id).await?;
        if listing.seller_id != seller_id {
            warn!(listing_id = %id, owner = %listing.seller_id, caller = %seller_id, "publish_listing forbidden: not owner");
            return Err(AppError::Forbidden(
                "You do not own this listing".to_string(),
            ));
        }
        let result = self
            .listing_repo
            .update(
                id,
                UpdateListing {
                    status: Some("active".to_string()),
                    ..Default::default()
                },
            )
            .await;
        if result.is_ok() {
            info!(listing_id = %id, "listing published");
        }
        result
    }

    #[instrument(skip(self), fields(listing_id = %id, seller_id = %seller_id))]
    pub async fn pause_listing(&self, id: Uuid, seller_id: Uuid) -> AppResult<Listing> {
        let listing = self.get_listing(id).await?;
        if listing.seller_id != seller_id {
            warn!(listing_id = %id, owner = %listing.seller_id, caller = %seller_id, "pause_listing forbidden: not owner");
            return Err(AppError::Forbidden(
                "You do not own this listing".to_string(),
            ));
        }
        let result = self
            .listing_repo
            .update(
                id,
                UpdateListing {
                    status: Some("paused".to_string()),
                    ..Default::default()
                },
            )
            .await;
        if result.is_ok() {
            info!(listing_id = %id, "listing paused");
        }
        result
    }
}

#[derive(Clone)]
pub struct CategoryService<C: CategoryRepository> {
    category_repo: C,
}

impl<C: CategoryRepository> CategoryService<C> {
    pub fn new(category_repo: C) -> Self {
        Self { category_repo }
    }

    #[instrument(skip(self))]
    pub async fn list_categories(&self) -> AppResult<Vec<Category>> {
        self.category_repo.find_all().await
    }

    #[instrument(skip(self), fields(kind))]
    pub async fn list_categories_by_kind(&self, kind: &str) -> AppResult<Vec<Category>> {
        self.category_repo.find_by_kind(kind).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listing::models::{ListingCondition, PaginatedResult};
    use async_trait::async_trait;
    use chrono::NaiveDateTime;
    use mockall::mock;

    mock! {
        pub ListingRepoMock {}

        #[async_trait]
        impl ListingRepository for ListingRepoMock {
            async fn create(&self, listing: NewListing) -> AppResult<Listing>;
            async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Listing>>;
            async fn find_all(&self, filters: ListingFilters, page: i64, per_page: i64) -> AppResult<PaginatedResult<Listing>>;
            async fn find_by_seller(&self, seller_id: Uuid, page: i64, per_page: i64) -> AppResult<PaginatedResult<Listing>>;
            async fn update(&self, id: Uuid, data: UpdateListing) -> AppResult<Listing>;
            async fn delete(&self, id: Uuid) -> AppResult<()>;
            async fn count_by_seller(&self, seller_id: Uuid) -> AppResult<i64>;
        }
    }

    mock! {
        pub CategoryRepoMock {}

        #[async_trait]
        impl CategoryRepository for CategoryRepoMock {
            async fn find_all(&self) -> AppResult<Vec<Category>>;
            async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Category>>;
            async fn find_by_kind(&self, kind: &str) -> AppResult<Vec<Category>>;
        }
    }

    fn dummy_listing(id: Uuid, seller_id: Uuid) -> Listing {
        Listing {
            id,
            seller_id,
            title: "Test".to_string(),
            description: "".to_string(),
            price_cents: 1000,
            category_id: Uuid::new_v4(),
            status: "draft".to_string(),
            condition: "handmade".to_string(),
            quantity: 1,
            created_at: NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
            updated_at: NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
        }
    }

    fn dummy_category(id: Uuid) -> Category {
        Category {
            id,
            name: "Test Category".to_string(),
            slug: "test-category".to_string(),
            kind: "craft".to_string(),
            created_at: NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
        }
    }

    fn service_with_mocks(
        listing_repo: MockListingRepoMock,
        category_repo: MockCategoryRepoMock,
    ) -> ListingService<MockListingRepoMock, MockCategoryRepoMock> {
        ListingService::new(listing_repo, category_repo)
    }

    #[tokio::test]
    async fn test_create_listing_title_empty() {
        let listing_repo = MockListingRepoMock::new();
        let category_repo = MockCategoryRepoMock::new();
        let service = service_with_mocks(listing_repo, category_repo);

        let req = CreateListingRequest {
            title: "  ".to_string(),
            description: "desc".to_string(),
            price_cents: 1000,
            category_id: Uuid::new_v4(),
            condition: ListingCondition::Handmade,
            quantity: Some(1),
        };

        let result = service.create_listing(Uuid::new_v4(), req).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_create_listing_price_zero() {
        let listing_repo = MockListingRepoMock::new();
        let category_repo = MockCategoryRepoMock::new();
        let service = service_with_mocks(listing_repo, category_repo);

        let req = CreateListingRequest {
            title: "Valid".to_string(),
            description: "".to_string(),
            price_cents: 0,
            category_id: Uuid::new_v4(),
            condition: ListingCondition::Handmade,
            quantity: Some(1),
        };

        let result = service.create_listing(Uuid::new_v4(), req).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_create_listing_category_not_found() {
        let listing_repo = MockListingRepoMock::new();
        let mut category_repo = MockCategoryRepoMock::new();

        let cat_id = Uuid::new_v4();
        category_repo
            .expect_find_by_id()
            .withf(move |id| *id == cat_id)
            .return_once(|_| Ok(None));

        let service = service_with_mocks(listing_repo, category_repo);

        let req = CreateListingRequest {
            title: "Valid".to_string(),
            description: "".to_string(),
            price_cents: 1000,
            category_id: cat_id,
            condition: ListingCondition::Handmade,
            quantity: Some(1),
        };

        let result = service.create_listing(Uuid::new_v4(), req).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_create_listing_defaults_status_to_draft() {
        let mut listing_repo = MockListingRepoMock::new();
        let mut category_repo = MockCategoryRepoMock::new();
        let cat_id = Uuid::new_v4();

        category_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_category(cat_id))));

        let expected_seller = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        listing_repo
            .expect_create()
            .withf(move |nl: &NewListing| {
                nl.seller_id == expected_seller && nl.status == "draft" && nl.quantity == 1
            })
            .return_once(move |_| Ok(dummy_listing(listing_id, expected_seller)));

        let service = service_with_mocks(listing_repo, category_repo);
        let seller_id = expected_seller;

        let req = CreateListingRequest {
            title: "Valid".to_string(),
            description: "".to_string(),
            price_cents: 1000,
            category_id: Uuid::new_v4(),
            condition: ListingCondition::Handmade,
            quantity: None,
        };

        let result = service.create_listing(seller_id, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_listing_returns_not_found() {
        let mut listing_repo = MockListingRepoMock::new();
        let listing_id = Uuid::new_v4();

        listing_repo.expect_find_by_id().return_once(|_| Ok(None));

        let category_repo = MockCategoryRepoMock::new();
        let service = service_with_mocks(listing_repo, category_repo);
        let result = service.get_listing(listing_id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_update_listing_forbidden() {
        let mut listing_repo = MockListingRepoMock::new();
        let owner_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();

        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, owner_id))));

        let category_repo = MockCategoryRepoMock::new();
        let service = service_with_mocks(listing_repo, category_repo);
        let result = service
            .update_listing(listing_id, Uuid::new_v4(), UpdateListingRequest::default())
            .await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_delete_listing_forbidden() {
        let mut listing_repo = MockListingRepoMock::new();
        let owner_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();

        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, owner_id))));

        let category_repo = MockCategoryRepoMock::new();
        let service = service_with_mocks(listing_repo, category_repo);
        let result = service.delete_listing(listing_id, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_publish_listing_changes_status() {
        let mut listing_repo = MockListingRepoMock::new();
        let seller_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();

        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, seller_id))));

        listing_repo
            .expect_update()
            .withf(move |id, data: &UpdateListing| {
                *id == listing_id && data.status == Some("active".to_string())
            })
            .return_once(|_, _| {
                Ok(Listing {
                    status: "active".to_string(),
                    ..dummy_listing(Uuid::new_v4(), Uuid::new_v4())
                })
            });

        let category_repo = MockCategoryRepoMock::new();
        let service = service_with_mocks(listing_repo, category_repo);
        let result = service.publish_listing(listing_id, seller_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, "active");
    }

    #[tokio::test]
    async fn test_pause_listing_changes_status() {
        let mut listing_repo = MockListingRepoMock::new();
        let seller_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();

        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, seller_id))));

        listing_repo
            .expect_update()
            .withf(move |id, data: &UpdateListing| {
                *id == listing_id && data.status == Some("paused".to_string())
            })
            .return_once(|_, _| {
                Ok(Listing {
                    status: "paused".to_string(),
                    ..dummy_listing(Uuid::new_v4(), Uuid::new_v4())
                })
            });

        let category_repo = MockCategoryRepoMock::new();
        let service = service_with_mocks(listing_repo, category_repo);
        let result = service.pause_listing(listing_id, seller_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, "paused");
    }

    #[tokio::test]
    async fn test_list_listings_forces_active_status() {
        let mut listing_repo = MockListingRepoMock::new();
        let seller_id = Uuid::new_v4();

        listing_repo
            .expect_find_all()
            .withf(
                move |filters: &ListingFilters, _page: &i64, _per_page: &i64| {
                    filters.status == Some("active".to_string())
                },
            )
            .return_once(move |_, _, _| {
                Ok(PaginatedResult {
                    items: vec![dummy_listing(Uuid::new_v4(), seller_id)],
                    total: 1,
                    page: 1,
                    per_page: 10,
                })
            });

        let category_repo = MockCategoryRepoMock::new();
        let service = service_with_mocks(listing_repo, category_repo);
        let result = service
            .list_listings(ListingFilters::default(), 1, 10)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().items.len(), 1);
    }
}
