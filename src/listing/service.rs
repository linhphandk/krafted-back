use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::listing::models::{
    Category, CreateListingRequest, ImageResponse, Listing, ListingFilters, NewListing,
    NewListingImage, PaginatedResult, UpdateListing, UpdateListingRequest,
};
use crate::listing::ports::{CategoryRepository, ListingImageRepository, ListingRepository};
use crate::shared::errors::{AppError, AppResult};
use crate::shared::image_processor;
use crate::shared::image_storage::ImageStorage;

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
            status: req
                .status
                .map(|s| s.to_string())
                .unwrap_or_else(|| "draft".to_string()),
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

#[derive(Clone)]
pub struct ListingImageService<R, S, L> {
    repo: R,
    storage: S,
    listing_repo: L,
    bucket: String,
}

impl<R: ListingImageRepository, S: ImageStorage, L: ListingRepository>
    ListingImageService<R, S, L>
{
    pub fn new(repo: R, storage: S, listing_repo: L, bucket: String) -> Self {
        Self {
            repo,
            storage,
            listing_repo,
            bucket,
        }
    }

    #[instrument(skip(self, files), fields(listing_id = %listing_id, seller_id = %seller_id, file_count = files.len()))]
    pub async fn upload_images(
        &self,
        listing_id: Uuid,
        seller_id: Uuid,
        files: Vec<(Vec<u8>, String, Option<i32>)>,
    ) -> AppResult<Vec<ImageResponse>> {
        let listing = self
            .listing_repo
            .find_by_id(listing_id)
            .await?
            .ok_or_else(|| {
                warn!(listing_id = %listing_id, "upload_images: listing not found");
                AppError::NotFound("Listing not found".to_string())
            })?;

        if listing.seller_id != seller_id {
            warn!(listing_id = %listing_id, owner = %listing.seller_id, caller = %seller_id, "upload_images forbidden: not owner");
            return Err(AppError::Forbidden(
                "You do not own this listing".to_string(),
            ));
        }

        let next = self.repo.next_position(listing_id).await?;
        let mut responses = Vec::with_capacity(files.len());

        for (i, (data, _content_type, position)) in files.into_iter().enumerate() {
            let pos = position.unwrap_or(next + i as i32);
            let processed = image_processor::process_image(&data)?;

            let image_id = Uuid::new_v4();
            let main_key = format!(
                "listings/{}/{}_{}.webp",
                listing_id,
                image_id,
                image_processor::MAIN_SIZE
            );
            let thumb_key = format!(
                "listings/{}/{}_{}.webp",
                listing_id,
                image_id,
                image_processor::THUMB_SIZE
            );

            let main_url = self
                .storage
                .upload(&self.bucket, &main_key, processed.main_webp, "image/webp")
                .await?;
            let thumb_url = self
                .storage
                .upload(&self.bucket, &thumb_key, processed.thumb_webp, "image/webp")
                .await?;

            let new_image = NewListingImage {
                listing_id,
                url: main_url,
                thumbnail_url: thumb_url,
                s3_key: main_key,
                position: pos,
            };

            let saved = self.repo.create(new_image).await?;
            responses.push(ImageResponse::from_image(&saved));
        }

        Ok(responses)
    }

    pub async fn list_images(&self, listing_id: Uuid) -> AppResult<Vec<ImageResponse>> {
        let images = self.repo.find_by_listing(listing_id).await?;
        Ok(images.iter().map(ImageResponse::from_image).collect())
    }

    #[instrument(skip(self), fields(listing_id = %listing_id))]
    pub async fn reorder_images(
        &self,
        listing_id: Uuid,
        seller_id: Uuid,
        image_ids: Vec<Uuid>,
    ) -> AppResult<()> {
        let listing = self
            .listing_repo
            .find_by_id(listing_id)
            .await?
            .ok_or_else(|| {
                warn!(listing_id = %listing_id, "reorder_images: listing not found");
                AppError::NotFound("Listing not found".to_string())
            })?;

        if listing.seller_id != seller_id {
            warn!(listing_id = %listing_id, owner = %listing.seller_id, caller = %seller_id, "reorder_images forbidden: not owner");
            return Err(AppError::Forbidden(
                "You do not own this listing".to_string(),
            ));
        }

        let existing = self.repo.find_by_listing(listing_id).await?;
        let existing_ids: std::collections::HashSet<Uuid> =
            existing.iter().map(|img| img.id).collect();

        for img_id in &image_ids {
            if !existing_ids.contains(img_id) {
                warn!(image_id = %img_id, "reorder_images: image not in listing");
                return Err(AppError::BadRequest(format!(
                    "Image {} does not belong to this listing",
                    img_id
                )));
            }
        }

        for (i, img_id) in image_ids.iter().enumerate() {
            self.repo.update_position(*img_id, i as i32).await?;
        }

        Ok(())
    }

    #[instrument(skip(self), fields(listing_id = %listing_id, image_id = %image_id))]
    pub async fn delete_image(
        &self,
        listing_id: Uuid,
        image_id: Uuid,
        seller_id: Uuid,
    ) -> AppResult<()> {
        let listing = self
            .listing_repo
            .find_by_id(listing_id)
            .await?
            .ok_or_else(|| {
                warn!(listing_id = %listing_id, "delete_image: listing not found");
                AppError::NotFound("Listing not found".to_string())
            })?;

        if listing.seller_id != seller_id {
            warn!(listing_id = %listing_id, owner = %listing.seller_id, caller = %seller_id, "delete_image forbidden: not owner");
            return Err(AppError::Forbidden(
                "You do not own this listing".to_string(),
            ));
        }

        let image = self.repo.find_by_id(image_id).await?.ok_or_else(|| {
            warn!(image_id = %image_id, "delete_image: image not found");
            AppError::NotFound("Image not found".to_string())
        })?;

        let thumb_key = image.s3_key.replace(
            &format!("{}.webp", image_processor::MAIN_SIZE),
            &format!("{}.webp", image_processor::THUMB_SIZE),
        );

        self.storage.delete(&self.bucket, &image.s3_key).await?;
        self.storage.delete(&self.bucket, &thumb_key).await?;
        self.repo.delete(image_id).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listing::models::{ListingCondition, ListingImage, ListingStatus, PaginatedResult};
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
            status: None,
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
            status: None,
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
            status: None,
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
            status: None,
        };

        let result = service.create_listing(seller_id, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_listing_with_active_status() {
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
                nl.seller_id == expected_seller && nl.status == "active" && nl.quantity == 1
            })
            .return_once(move |_| Ok(dummy_listing(listing_id, expected_seller)));

        let service = service_with_mocks(listing_repo, category_repo);
        let seller_id = expected_seller;

        let req = CreateListingRequest {
            title: "Active Item".to_string(),
            description: "".to_string(),
            price_cents: 2000,
            category_id: Uuid::new_v4(),
            condition: ListingCondition::New,
            quantity: None,
            status: Some(ListingStatus::Active),
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

    mock! {
        pub ListingImageRepoMock {}
        #[async_trait]
        impl ListingImageRepository for ListingImageRepoMock {
            async fn create(&self, image: NewListingImage) -> AppResult<ListingImage>;
            async fn find_by_listing(&self, listing_id: Uuid) -> AppResult<Vec<ListingImage>>;
            async fn find_by_id(&self, id: Uuid) -> AppResult<Option<ListingImage>>;
            async fn delete(&self, id: Uuid) -> AppResult<()>;
            async fn next_position(&self, listing_id: Uuid) -> AppResult<i32>;
            async fn update_position(&self, id: Uuid, position: i32) -> AppResult<()>;
        }
    }

    fn dummy_listing_image(id: Uuid, listing_id: Uuid, position: i32) -> ListingImage {
        ListingImage {
            id,
            listing_id,
            url: format!("http://mock/bucket/listings/{}/{}_800.webp", listing_id, id),
            thumbnail_url: format!("http://mock/bucket/listings/{}/{}_400.webp", listing_id, id),
            s3_key: format!("listings/{}/{}_800.webp", listing_id, id),
            position,
            created_at: NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
        }
    }

    fn test_jpeg_bytes() -> Vec<u8> {
        let mut img = image::RgbImage::new(100, 100);
        for x in 0..100 {
            for y in 0..100 {
                img.put_pixel(x, y, image::Rgb([x as u8, y as u8, 128]));
            }
        }
        let mut bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
        bytes
    }

    fn image_service_with_mocks(
        image_repo: MockListingImageRepoMock,
        storage: crate::shared::image_storage::test::MockImageStorage,
        listing_repo: MockListingRepoMock,
    ) -> ListingImageService<
        MockListingImageRepoMock,
        crate::shared::image_storage::test::MockImageStorage,
        MockListingRepoMock,
    > {
        ListingImageService::new(image_repo, storage, listing_repo, "test-bucket".to_string())
    }

    #[tokio::test]
    async fn test_upload_images_success() {
        let seller_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let mut listing_repo = MockListingRepoMock::new();
        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, seller_id))));

        let mut image_repo = MockListingImageRepoMock::new();
        image_repo.expect_next_position().return_once(|_| Ok(0));

        let image_id = Uuid::new_v4();
        image_repo.expect_create().return_once(move |nl| {
            Ok(ListingImage {
                id: image_id,
                listing_id: nl.listing_id,
                url: nl.url,
                thumbnail_url: nl.thumbnail_url,
                s3_key: nl.s3_key,
                position: nl.position,
                created_at: NaiveDateTime::parse_from_str(
                    "2026-01-01 00:00:00",
                    "%Y-%m-%d %H:%M:%S",
                )
                .unwrap(),
            })
        });

        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let jpeg = test_jpeg_bytes();
        let result = service
            .upload_images(
                listing_id,
                seller_id,
                vec![(jpeg, "image/jpeg".to_string(), None)],
            )
            .await;

        assert!(result.is_ok());
        let images = result.unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].position, 0);
    }

    #[tokio::test]
    async fn test_upload_images_listing_not_found() {
        let listing_repo = {
            let mut repo = MockListingRepoMock::new();
            repo.expect_find_by_id().return_once(|_| Ok(None));
            repo
        };
        let image_repo = MockListingImageRepoMock::new();
        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service
            .upload_images(Uuid::new_v4(), Uuid::new_v4(), vec![])
            .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_upload_images_forbidden() {
        let owner_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let mut listing_repo = MockListingRepoMock::new();
        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, owner_id))));

        let image_repo = MockListingImageRepoMock::new();
        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service
            .upload_images(listing_id, Uuid::new_v4(), vec![])
            .await;

        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_upload_images_custom_position() {
        let seller_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let mut listing_repo = MockListingRepoMock::new();
        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, seller_id))));

        let mut image_repo = MockListingImageRepoMock::new();
        image_repo.expect_next_position().return_once(|_| Ok(0));

        let image_id = Uuid::new_v4();
        image_repo
            .expect_create()
            .withf(move |nl: &NewListingImage| nl.position == 5)
            .return_once(move |nl| {
                Ok(ListingImage {
                    id: image_id,
                    listing_id: nl.listing_id,
                    url: nl.url,
                    thumbnail_url: nl.thumbnail_url,
                    s3_key: nl.s3_key,
                    position: nl.position,
                    created_at: NaiveDateTime::parse_from_str(
                        "2026-01-01 00:00:00",
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .unwrap(),
                })
            });

        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let jpeg = test_jpeg_bytes();
        let result = service
            .upload_images(
                listing_id,
                seller_id,
                vec![(jpeg, "image/jpeg".to_string(), Some(5))],
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].position, 5);
    }

    #[tokio::test]
    async fn test_upload_images_multiple_auto_position() {
        let seller_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let mut listing_repo = MockListingRepoMock::new();
        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, seller_id))));

        let mut image_repo = MockListingImageRepoMock::new();
        image_repo.expect_next_position().return_once(|_| Ok(3));

        let call_count = std::sync::atomic::AtomicUsize::new(0);
        image_repo.expect_create().returning(move |nl| {
            let pos = nl.position;
            let idx = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let image_id = if idx == 0 {
                Uuid::new_v4()
            } else {
                Uuid::new_v4()
            };
            Ok(ListingImage {
                id: image_id,
                listing_id: nl.listing_id,
                url: nl.url,
                thumbnail_url: nl.thumbnail_url,
                s3_key: nl.s3_key,
                position: pos,
                created_at: NaiveDateTime::parse_from_str(
                    "2026-01-01 00:00:00",
                    "%Y-%m-%d %H:%M:%S",
                )
                .unwrap(),
            })
        });

        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let jpeg = test_jpeg_bytes();
        let result = service
            .upload_images(
                listing_id,
                seller_id,
                vec![
                    (jpeg.clone(), "image/jpeg".to_string(), None),
                    (jpeg, "image/jpeg".to_string(), None),
                ],
            )
            .await;

        assert!(result.is_ok());
        let images = result.unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].position, 3);
        assert_eq!(images[1].position, 4);
    }

    #[tokio::test]
    async fn test_list_images_success() {
        let listing_id = Uuid::new_v4();
        let mut image_repo = MockListingImageRepoMock::new();
        let img1 = dummy_listing_image(Uuid::new_v4(), listing_id, 0);
        let img2 = dummy_listing_image(Uuid::new_v4(), listing_id, 1);
        let expected = vec![img1, img2];
        let clone = expected.clone();
        image_repo
            .expect_find_by_listing()
            .return_once(move |_| Ok(clone));

        let listing_repo = MockListingRepoMock::new();
        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service.list_images(listing_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), expected.len());
    }

    #[tokio::test]
    async fn test_reorder_images_success() {
        let seller_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let img_a = dummy_listing_image(Uuid::new_v4(), listing_id, 0);
        let img_b = dummy_listing_image(Uuid::new_v4(), listing_id, 1);
        let img_c = dummy_listing_image(Uuid::new_v4(), listing_id, 2);

        let mut listing_repo = MockListingRepoMock::new();
        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, seller_id))));

        let mut image_repo = MockListingImageRepoMock::new();
        let existing = vec![img_a.clone(), img_b.clone(), img_c.clone()];
        image_repo
            .expect_find_by_listing()
            .return_once(move |_| Ok(existing));

        let order = std::sync::Mutex::new(Vec::new());
        image_repo
            .expect_update_position()
            .returning(move |id, pos| {
                order.lock().unwrap().push((id, pos));
                Ok(())
            });

        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service
            .reorder_images(listing_id, seller_id, vec![img_c.id, img_b.id, img_a.id])
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reorder_images_listing_not_found() {
        let mut listing_repo = MockListingRepoMock::new();
        listing_repo.expect_find_by_id().return_once(|_| Ok(None));

        let image_repo = MockListingImageRepoMock::new();
        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service
            .reorder_images(Uuid::new_v4(), Uuid::new_v4(), vec![])
            .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_reorder_images_forbidden() {
        let owner_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let mut listing_repo = MockListingRepoMock::new();
        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, owner_id))));

        let image_repo = MockListingImageRepoMock::new();
        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service
            .reorder_images(listing_id, Uuid::new_v4(), vec![])
            .await;

        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_reorder_images_invalid_image() {
        let seller_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let img_a = dummy_listing_image(Uuid::new_v4(), listing_id, 0);

        let mut listing_repo = MockListingRepoMock::new();
        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, seller_id))));

        let mut image_repo = MockListingImageRepoMock::new();
        image_repo
            .expect_find_by_listing()
            .return_once(move |_| Ok(vec![img_a]));

        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service
            .reorder_images(listing_id, seller_id, vec![Uuid::new_v4()])
            .await;

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_delete_image_success() {
        let seller_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let image_id = Uuid::new_v4();
        let img = dummy_listing_image(image_id, listing_id, 0);

        let mut listing_repo = MockListingRepoMock::new();
        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, seller_id))));

        let mut image_repo = MockListingImageRepoMock::new();
        let img_clone = img.clone();
        image_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(img_clone)));
        image_repo.expect_delete().return_once(|_| Ok(()));

        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service.delete_image(listing_id, image_id, seller_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_image_listing_not_found() {
        let mut listing_repo = MockListingRepoMock::new();
        listing_repo.expect_find_by_id().return_once(|_| Ok(None));

        let image_repo = MockListingImageRepoMock::new();
        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service
            .delete_image(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_image_forbidden() {
        let owner_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let mut listing_repo = MockListingRepoMock::new();
        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, owner_id))));

        let image_repo = MockListingImageRepoMock::new();
        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service
            .delete_image(listing_id, Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_delete_image_not_found() {
        let seller_id = Uuid::new_v4();
        let listing_id = Uuid::new_v4();
        let mut listing_repo = MockListingRepoMock::new();
        listing_repo
            .expect_find_by_id()
            .return_once(move |_| Ok(Some(dummy_listing(listing_id, seller_id))));

        let mut image_repo = MockListingImageRepoMock::new();
        image_repo.expect_find_by_id().return_once(|_| Ok(None));

        let storage = crate::shared::image_storage::test::MockImageStorage::new();
        let service = image_service_with_mocks(image_repo, storage, listing_repo);

        let result = service
            .delete_image(listing_id, Uuid::new_v4(), seller_id)
            .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
