use crate::shared::db::DbPool;
use crate::user::repository::DieselUserRepository;
use crate::user::service::UserService;

#[derive(Clone)]
pub struct UserAppState {
    pub service: UserService<DieselUserRepository>,
}

impl UserAppState {
    pub fn new(pool: DbPool) -> Self {
        let repo = DieselUserRepository::new(pool);
        let service = UserService::new(repo);
        Self { service }
    }
}
