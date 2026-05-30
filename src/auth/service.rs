use crate::shared::errors::{AppError, AppResult};
use crate::user::models::{NewUser, User};
use crate::user::ports::UserRepository;

#[derive(Clone)]
pub struct AuthService<R: UserRepository> {
    repo: R,
}

impl<R: UserRepository> AuthService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn register(&self, email: String, name: String, password: String) -> AppResult<User> {
        if email.is_empty() {
            return Err(AppError::BadRequest("Email cannot be empty".to_string()));
        }
        if name.is_empty() {
            return Err(AppError::BadRequest("Name cannot be empty".to_string()));
        }
        if password.len() < 8 {
            return Err(AppError::BadRequest(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
            tracing::error!("Password hashing error: {:?}", e);
            AppError::Internal
        })?;

        let new_user = NewUser {
            email,
            name,
            password_hash,
        };

        self.repo.create(new_user).await
    }
}
