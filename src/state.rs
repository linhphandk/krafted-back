use crate::shared::db::DbPool;
use crate::user::repository::DieselUserRepository;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: crate::auth::service::AuthService<DieselUserRepository>,
}

impl AppState {
    pub fn new(pool: DbPool) -> Self {
        let repo = DieselUserRepository::new(pool);
        let auth_service = crate::auth::service::AuthService::new(repo);
        Self { auth_service }
    }
}
