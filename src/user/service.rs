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

    pub async fn create(&self, new_user: NewUser) -> AppResult<User> {
        self.repo.create(new_user).await
    }

    pub async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        self.repo.find_by_email(email).await
    }

    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        self.repo.find_by_id(id).await
    }
}
