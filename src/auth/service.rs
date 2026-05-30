use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::auth::models::Tokens;
use crate::auth::ports::AuthProvider;
use crate::session::models::NewSession;
use crate::session::ports::SessionRepository;
use crate::shared::errors::{AppError, AppResult};
use crate::user::models::{NewUser, User};
use crate::user::ports::UserRepository;

#[derive(Clone)]
pub struct AuthService<A: AuthProvider, R: UserRepository, S: SessionRepository> {
    auth_provider: A,
    user_repo: R,
    session_repo: S,
    refresh_token_expiry_days: i64,
}

impl<A: AuthProvider, R: UserRepository, S: SessionRepository> AuthService<A, R, S> {
    pub fn new(auth_provider: A, user_repo: R, session_repo: S, refresh_token_expiry_days: i64) -> Self {
        Self {
            auth_provider,
            user_repo,
            session_repo,
            refresh_token_expiry_days,
        }
    }

    pub async fn register(
        &self,
        email: String,
        name: String,
        password: String,
    ) -> AppResult<(User, Tokens)> {
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

        let (user_info, tokens) = self.auth_provider.register(&email, &name, &password).await?;

        let new_user = NewUser {
            email,
            name,
            password_hash: user_info.password_hash,
        };

        let user = self.user_repo.create(new_user).await?;
        Ok((user, tokens))
    }

    pub async fn login(&self, email: String, password: String) -> AppResult<(User, Tokens)> {
        let user = self.user_repo.find_by_email(&email).await?;
        let user = user.ok_or(AppError::BadRequest("Invalid email or password".to_string()))?;

        let (tokens, _user_info) = self
            .auth_provider
            .login(&email, &password, &user.password_hash)
            .await?;

        let refresh_token = Uuid::new_v4().to_string();
        let expires_at = Utc::now()
            .checked_add_signed(Duration::days(self.refresh_token_expiry_days))
            .ok_or(AppError::Internal)?
            .naive_utc();

        let session = self
            .session_repo
            .create(NewSession {
                user_id: user.id,
                refresh_token: refresh_token.clone(),
                expires_at,
            })
            .await?;

        let tokens = Tokens {
            access_token: tokens.access_token,
            refresh_token: session.refresh_token,
            id_token: String::new(),
            expires_in: tokens.expires_in,
        };

        Ok((user, tokens))
    }

    pub async fn logout(&self, refresh_token: String) -> AppResult<()> {
        self.session_repo.revoke(&refresh_token).await
    }
}
