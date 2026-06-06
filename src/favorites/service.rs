use tracing::{debug, instrument};
use uuid::Uuid;

use crate::favorites::models::{Favorite, NewFavorite};
use crate::favorites::ports::{FavoriteRepository, PaginatedFavorites};
use crate::listing::ports::ListingRepository;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct FavoriteService<R: FavoriteRepository, L: ListingRepository> {
    repo: R,
    listing_repo: L,
}

impl<R: FavoriteRepository, L: ListingRepository> FavoriteService<R, L> {
    pub fn new(repo: R, listing_repo: L) -> Self {
        Self { repo, listing_repo }
    }

    #[instrument(skip(self), fields(user_id = %user_id, listing_id = %listing_id))]
    pub async fn add(&self, user_id: Uuid, listing_id: Uuid) -> AppResult<Favorite> {
        debug!("add favorite");

        let listing = self
            .listing_repo
            .find_by_id(listing_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Listing not found".to_string()))?;

        if listing.seller_id == user_id {
            return Err(AppError::BadRequest(
                "Cannot favorite your own listing".to_string(),
            ));
        }

        let favorite = NewFavorite {
            user_id,
            listing_id,
        };
        self.repo.create(favorite).await
    }

    #[instrument(skip(self), fields(user_id = %user_id, listing_id = %listing_id))]
    pub async fn remove(&self, user_id: Uuid, listing_id: Uuid) -> AppResult<()> {
        debug!("remove favorite");
        let removed = self.repo.delete(user_id, listing_id).await?;
        if !removed {
            return Err(AppError::NotFound("Favorite not found".to_string()));
        }
        Ok(())
    }

    #[instrument(skip(self), fields(user_id = %user_id))]
    pub async fn list(
        &self,
        user_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> AppResult<PaginatedFavorites> {
        debug!("list favorites");
        self.repo.find_by_user(user_id, page, per_page).await
    }

    #[instrument(skip(self), fields(user_id = %user_id, listing_id = %listing_id))]
    pub async fn check(&self, user_id: Uuid, listing_id: Uuid) -> AppResult<bool> {
        debug!("check favorite");
        self.repo.check(user_id, listing_id).await
    }
}
