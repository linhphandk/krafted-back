use crate::shared::db::DbPool;
use crate::user::repository::DieselUserRepository;
use crate::user::service::UserService;

#[derive(Clone)]
pub struct AppState {
    pub user_service: UserService<DieselUserRepository>,
    pub auth_service: crate::auth::service::AuthService<DieselUserRepository>,
}

impl AppState {
    pub fn new(pool: DbPool) -> Self {
        let repo = DieselUserRepository::new(pool);
        let user_service = UserService::new(repo.clone());
        let auth_service = crate::auth::service::AuthService::new(repo);
        Self {
            user_service,
            auth_service,
        }
    }
}
