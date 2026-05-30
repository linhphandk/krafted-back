use crate::auth::provider::LocalAuthProvider;
use crate::shared::db::DbPool;
use crate::user::repository::DieselUserRepository;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: crate::auth::service::AuthService<LocalAuthProvider, DieselUserRepository>,
}

impl AppState {
    pub fn new(pool: DbPool) -> Self {
        let auth_provider = LocalAuthProvider::new();
        let user_repo = DieselUserRepository::new(pool);
        let auth_service = crate::auth::service::AuthService::new(auth_provider, user_repo);
        Self { auth_service }
    }
}
