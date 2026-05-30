use async_trait::async_trait;

use crate::shared::errors::AppResult;
use crate::user::models::{NewUser, User};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: NewUser) -> AppResult<User>;
    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
}
