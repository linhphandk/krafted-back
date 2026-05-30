use crate::shared::db::DbPool;
use crate::user::repository::DieselUserRepository;
use crate::user::service::UserService;

pub type UserRepo = DieselUserRepository;
pub type UserSvc = UserService<UserRepo>;

#[derive(Clone)]
pub struct UserAppState {
    pub service: UserSvc,
}

impl UserAppState {
    pub fn new(pool: DbPool) -> Self {
        let repo = UserRepo::new(pool);
        let service = UserSvc::new(repo);
        Self { service }
    }
}
