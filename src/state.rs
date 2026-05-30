use crate::auth::provider::LocalAuthProvider;
use crate::shared::db::DbPool;
use crate::user::repository::DieselUserRepository;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: crate::auth::service::AuthService<LocalAuthProvider, DieselUserRepository>,
}

impl AppState {
    pub fn new(pool: DbPool, jwt_secret: String, jwt_expiry_minutes: u64) -> Self {
        let auth_provider = LocalAuthProvider::new(jwt_secret, jwt_expiry_minutes);
        let user_repo = DieselUserRepository::new(pool);
        let auth_service = crate::auth::service::AuthService::new(auth_provider, user_repo);
        Self { auth_service }
    }
}
