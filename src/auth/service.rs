use crate::auth::ports::AuthProvider;
use crate::shared::errors::{AppError, AppResult};
use crate::user::models::{NewUser, User};
use crate::user::ports::UserRepository;

#[derive(Clone)]
pub struct AuthService<A: AuthProvider, R: UserRepository> {
    auth_provider: A,
    user_repo: R,
}

impl<A: AuthProvider, R: UserRepository> AuthService<A, R> {
    pub fn new(auth_provider: A, user_repo: R) -> Self {
        Self {
            auth_provider,
            user_repo,
        }
    }

    pub async fn register(
        &self,
        email: String,
        name: String,
        password: String,
    ) -> AppResult<User> {
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

        let _user_info = self.auth_provider.register(&email, &name, &password).await?;

        let new_user = NewUser {
            email,
            name,
            password_hash: String::new(),
        };

        self.user_repo.create(new_user).await
    }
}
