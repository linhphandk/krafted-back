use tracing::{debug, instrument};
use uuid::Uuid;

use crate::shared::errors::AppResult;
use crate::user::models::{NewUser, User};
use crate::user::ports::UserRepository;

#[derive(Clone)]
pub struct UserService<R: UserRepository> {
    repo: R,
}

impl<R: UserRepository> UserService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    #[instrument(skip(self, new_user), fields(email = %new_user.email))]
    pub async fn create(&self, new_user: NewUser) -> AppResult<User> {
        debug!("create user");
        self.repo.create(new_user).await
    }

    #[instrument(skip(self), fields(email = %email))]
    pub async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        debug!(email = %email, "find_by_email");
        self.repo.find_by_email(email).await
    }

    #[instrument(skip(self), fields(user_id = %id))]
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        debug!(user_id = %id, "find_by_id");
        self.repo.find_by_id(id).await
    }
}
