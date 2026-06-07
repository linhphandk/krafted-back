use crate::auth::provider::LocalAuthProvider;
use crate::favorites::repository::DieselFavoriteRepository;
use crate::favorites::service::FavoritesService;
use crate::listing::repository::{
    DieselCategoryRepository, DieselListingImageRepository, DieselListingRepository,
};
use crate::listing::service::{CategoryService, ListingImageService, ListingService};
use crate::rbac::repository::DieselRbacRepository;
use crate::rbac::service::RbacService;
use crate::session::repository::DieselSessionRepository;
use crate::shared::db::DbPool;
use crate::shared::image_storage::S3ImageStorage;
use crate::user::repository::DieselUserRepository;
use crate::user::service::UserService;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: crate::auth::service::AuthService<
        LocalAuthProvider,
        DieselUserRepository,
        DieselSessionRepository,
    >,
    pub listing_service: ListingService<DieselListingRepository, DieselCategoryRepository>,
    pub listing_image_service:
        ListingImageService<DieselListingImageRepository, S3ImageStorage, DieselListingRepository>,
    pub category_service: CategoryService<DieselCategoryRepository>,
    pub user_service: UserService<DieselUserRepository>,
    pub favorites_service: FavoritesService<DieselFavoriteRepository, DieselListingRepository>,
}

impl AppState {
    pub fn new(
        pool: DbPool,
        jwt_secret: String,
        jwt_expiry_minutes: u64,
        image_storage: S3ImageStorage,
        bucket: String,
        s3_public_url: Option<String>,
    ) -> Self {
        let auth_provider = LocalAuthProvider::new(jwt_secret.clone(), jwt_expiry_minutes);
        let user_repo = DieselUserRepository::new(pool.clone());
        let user_repo_for_auth = user_repo.clone();
        let session_repo = DieselSessionRepository::new(pool.clone());
        let rbac_repo = DieselRbacRepository::new(pool.clone());
        let rbac_service = Arc::new(RbacService::new(Arc::new(rbac_repo)));
        let auth_service = crate::auth::service::AuthService::new(
            auth_provider,
            user_repo_for_auth,
            session_repo,
            7,
            rbac_service,
        );

        let category_repo = DieselCategoryRepository::new(pool.clone());
        let listing_repo = DieselListingRepository::new(pool.clone());
        let listing_image_repo = DieselListingImageRepository::new(pool.clone());

        let listing_repo_for_images = listing_repo.clone();
        let listing_service = ListingService::new(listing_repo.clone(), category_repo.clone());
        let listing_image_service = ListingImageService::new(
            listing_image_repo,
            image_storage,
            listing_repo_for_images,
            bucket,
            s3_public_url,
        );
        let category_service = CategoryService::new(category_repo);

        let favorite_repo = DieselFavoriteRepository::new(pool.clone());
        let favorites_service =
            FavoritesService::new(favorite_repo, listing_repo.clone());

        let user_service = UserService::new(user_repo);

        Self {
            auth_service,
            listing_service,
            listing_image_service,
            category_service,
            user_service,
            favorites_service,
        }
    }
}
