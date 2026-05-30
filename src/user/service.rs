use crate::shared::errors::{AppError, AppResult};
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

    pub async fn create_user(&self, email: String, name: String) -> AppResult<User> {
        if email.is_empty() {
            return Err(AppError::BadRequest("Email cannot be empty".to_string()));
        }
        if name.is_empty() {
            return Err(AppError::BadRequest("Name cannot be empty".to_string()));
        }

        let new_user = NewUser {
            email,
            name,
            password_hash: String::new(),
        };
        self.repo.create(new_user).await
    }
}
