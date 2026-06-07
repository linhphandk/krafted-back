use tracing::{debug, instrument};
use uuid::Uuid;

use crate::shared::errors::{AppError, AppResult};
use crate::user::models::{NewUser, UpdateUser, User};
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

        if new_user.email.is_empty() {
            return Err(AppError::BadRequest("Email cannot be empty".to_string()));
        }
        if new_user.name.is_empty() {
            return Err(AppError::BadRequest("Name cannot be empty".to_string()));
        }

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

    #[instrument(skip(self), fields(user_id = %id))]
    pub async fn update_profile(&self, id: Uuid, data: UpdateUser) -> AppResult<User> {
        debug!(user_id = %id, "update_profile");

        if let Some(ref name) = data.name {
            if name.trim().is_empty() {
                return Err(AppError::BadRequest("Name cannot be empty".to_string()));
            }
        }

        if let Some(ref email) = data.email {
            if email.trim().is_empty() {
                return Err(AppError::BadRequest("Email cannot be empty".to_string()));
            }
            if let Some(existing) = self.repo.find_by_email(email).await? {
                if existing.id != id {
                    return Err(AppError::BadRequest("Email already in use".to_string()));
                }
            }
        }

        self.repo.update(id, data).await
    }

    #[instrument(skip(self), fields(user_id = %id))]
    pub async fn update_password_hash(&self, id: Uuid, password_hash: String) -> AppResult<()> {
        debug!(user_id = %id, "update_password_hash");
        self.repo.update_password_hash(id, password_hash).await
    }
}
