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
        if existing.is_some() {
            info!(user_id = %user_id, listing_id = %listing_id, "add_favorite: already exists");
            return Ok(FavoriteResponse::from_favorite(&existing.unwrap()));
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
    pub async fn remove_favorite(
        &self,
        user_id: Uuid,
        listing_id: Uuid,
    ) -> AppResult<()> {
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

        let mut active_items = Vec::new();
        for fav in items {
            match self.listing_repo.find_by_id(fav.listing_id).await {
                Ok(Some(listing)) if listing.status == "active" => {
                    active_items.push(FavoriteResponse::from_favorite(&fav));
                }
                _ => {}
            }
        }

        let active_total = active_items.len() as i64;
        Ok(PaginatedResult {
            items: active_items,
            total: active_total,
            page,
            per_page,
        })
    }
}
