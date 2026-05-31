use crate::auth::provider::LocalAuthProvider;
use crate::rbac::repository::DieselRbacRepository;
use crate::rbac::service::RbacService;
use crate::session::repository::DieselSessionRepository;
use crate::shared::db::DbPool;
use crate::user::repository::DieselUserRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: crate::auth::service::AuthService<
        LocalAuthProvider,
        DieselUserRepository,
        DieselSessionRepository,
    >,
}

impl AppState {
    pub fn new(pool: DbPool, jwt_secret: String, jwt_expiry_minutes: u64) -> Self {
        let auth_provider = LocalAuthProvider::new(jwt_secret.clone(), jwt_expiry_minutes);
        let user_repo = DieselUserRepository::new(pool.clone());
        let session_repo = DieselSessionRepository::new(pool.clone());
        let rbac_repo = DieselRbacRepository::new(pool.clone());
        let rbac_service = Arc::new(RbacService::new(Arc::new(rbac_repo)));
        let auth_service = crate::auth::service::AuthService::new(
            auth_provider,
            user_repo,
            session_repo,
            7,
            rbac_service,
        );
        Self { auth_service }
    }
}
