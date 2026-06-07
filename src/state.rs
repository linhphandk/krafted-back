use crate::auth::email::SmtpEmailProvider;
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
    pub email_provider: SmtpEmailProvider,
}

impl AppState {
    pub fn new(
        pool: DbPool,
        jwt_secret: String,
        jwt_expiry_minutes: u64,
        image_storage: S3ImageStorage,
        bucket: String,
        s3_public_url: Option<String>,
        smtp_host: String,
        smtp_port: u16,
        smtp_user: String,
        smtp_password: String,
        smtp_from_email: String,
        smtp_from_name: String,
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

        let listing_service = ListingService::new(listing_repo.clone(), category_repo.clone());
        let listing_image_service = ListingImageService::new(
            listing_image_repo,
            image_storage,
            listing_repo.clone(),
            bucket,
            s3_public_url.clone(),
        );
        let category_service = CategoryService::new(category_repo);

        let favorite_repo = DieselFavoriteRepository::new(pool);
        let favorites_service =
            FavoritesService::new(favorite_repo, listing_repo, s3_public_url);

        let user_service = UserService::new(user_repo);

        let email_provider = SmtpEmailProvider::new(
            &smtp_host,
            smtp_port,
            &smtp_user,
            &smtp_password,
            &smtp_from_email,
            &smtp_from_name,
        );

        Self {
            auth_service,
            listing_service,
            listing_image_service,
            category_service,
            user_service,
            favorites_service,
            email_provider,
        }
    }
}
