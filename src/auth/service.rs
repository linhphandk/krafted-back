use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::models::Tokens;
use crate::auth::ports::AuthProvider;
use crate::rbac::service::RbacService;
use crate::session::models::NewSession;
use crate::session::ports::SessionRepository;
use crate::shared::errors::{AppError, AppResult};
use crate::user::models::{NewUser, User};
use crate::user::ports::UserRepository;
use crate::user::service::UserService;

#[derive(Clone)]
pub struct AuthService<A: AuthProvider, R: UserRepository, S: SessionRepository> {
    auth_provider: A,
    user_service: UserService<R>,
    session_repo: S,
    refresh_token_expiry_days: i64,
    rbac_service: Arc<RbacService>,
}

impl<A: AuthProvider, R: UserRepository, S: SessionRepository> AuthService<A, R, S> {
    pub fn new(
        auth_provider: A,
        user_repo: R,
        session_repo: S,
        refresh_token_expiry_days: i64,
        rbac_service: Arc<RbacService>,
    ) -> Self {
        Self {
            auth_provider,
            user_service: UserService::new(user_repo),
            session_repo,
            refresh_token_expiry_days,
            rbac_service,
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

        let user = self.user_service.create(new_user).await?;
        self.rbac_service.assign_default_role(user.id).await?;
        Ok((user, tokens))
    }

    pub async fn login(&self, email: String, password: String) -> AppResult<(User, Tokens)> {
        let user = self.user_service.find_by_email(&email).await?;
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

    pub async fn refresh_token(&self, refresh_token: String) -> AppResult<(User, Tokens)> {
        let session = self.session_repo.find_by_token(&refresh_token).await?;
        let session = session.ok_or(AppError::BadRequest("Invalid refresh token".to_string()))?;

        if session.expires_at < Utc::now().naive_utc() {
            self.session_repo.revoke(&refresh_token).await?;
            return Err(AppError::BadRequest("Refresh token expired".to_string()));
        }

        let user = self.user_service.find_by_id(session.user_id).await?;
        let user = user.ok_or(AppError::Internal)?;

        let new_access_token = self
            .auth_provider
            .generate_access_token(&user.id.to_string(), &user.email)
            .await?;

        self.session_repo.revoke(&refresh_token).await?;

        let new_refresh_token = Uuid::new_v4().to_string();
        let expires_at = Utc::now()
            .checked_add_signed(Duration::days(self.refresh_token_expiry_days))
            .ok_or(AppError::Internal)?
            .naive_utc();

        let new_session = self
            .session_repo
            .create(NewSession {
                user_id: user.id,
                refresh_token: new_refresh_token.clone(),
                expires_at,
            })
            .await?;

        let tokens = Tokens {
            access_token: new_access_token,
            refresh_token: new_session.refresh_token,
            id_token: String::new(),
            expires_in: self.auth_provider.token_expiry_seconds(),
        };

        Ok((user, tokens))
    }

    pub async fn get_current_user(&self, access_token: String) -> AppResult<User> {
        let user_info = self.auth_provider.introspect_token(&access_token).await?;
        let user = self.user_service.find_by_email(&user_info.email).await?;
        user.ok_or(AppError::BadRequest("User not found".to_string()))
    }
}
