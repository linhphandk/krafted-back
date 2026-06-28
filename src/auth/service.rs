use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::auth::email::EmailProvider;
use crate::auth::models::{NewPasswordReset, Tokens};
use crate::auth::ports::{AuthProvider, PasswordResetRepository};
use crate::rbac::service::RbacService;
use crate::session::models::NewSession;
use crate::session::ports::SessionRepository;
use crate::shared::errors::{AppError, AppResult};
use crate::user::models::{NewUser, User};
use crate::user::ports::UserRepository;
use crate::user::service::UserService;

#[derive(Clone)]
pub struct AuthService<
    A: AuthProvider,
    R: UserRepository,
    S: SessionRepository,
    P: PasswordResetRepository,
    E: EmailProvider,
> {
    auth_provider: A,
    user_service: UserService<R>,
    session_repo: S,
    password_reset_repo: P,
    email_provider: E,
    refresh_token_expiry_days: i64,
    rbac_service: Arc<RbacService>,
    frontend_url: String,
}

impl<
        A: AuthProvider,
        R: UserRepository,
        S: SessionRepository,
        P: PasswordResetRepository,
        E: EmailProvider,
    > AuthService<A, R, S, P, E>
{
    pub fn new(
        auth_provider: A,
        user_repo: R,
        session_repo: S,
        password_reset_repo: P,
        email_provider: E,
        refresh_token_expiry_days: i64,
        rbac_service: Arc<RbacService>,
        frontend_url: String,
    ) -> Self {
        Self {
            auth_provider,
            user_service: UserService::new(user_repo),
            session_repo,
            password_reset_repo,
            email_provider,
            refresh_token_expiry_days,
            rbac_service,
            frontend_url,
        }
    }

    #[instrument(skip(self), fields(email = %email))]
    pub async fn register(
        &self,
        email: String,
        name: String,
        password: String,
    ) -> AppResult<(User, Tokens)> {
        if email.is_empty() {
            warn!("register rejected: empty email");
            return Err(AppError::BadRequest("Email cannot be empty".to_string()));
        }
        if name.is_empty() {
            warn!("register rejected: empty name");
            return Err(AppError::BadRequest("Name cannot be empty".to_string()));
        }
        if password.len() < 8 {
            warn!("register rejected: short password");
            return Err(AppError::BadRequest(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        let (user_info, tokens) = self
            .auth_provider
            .register(&email, &name, &password)
            .await?;

        let new_user = NewUser {
            email,
            name,
            password_hash: user_info.password_hash,
        };

        let user = self.user_service.create(new_user).await?;
        info!(user_id = %user.id, "user registered");
        self.rbac_service.assign_default_role(user.id).await?;

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

    #[instrument(skip(self), fields(email = %email))]
    pub async fn login(&self, email: String, password: String) -> AppResult<(User, Tokens)> {
        let user = self.user_service.find_by_email(&email).await?;
        let user = user.ok_or_else(|| {
            warn!(email = %email, "login failed: user not found");
            AppError::BadRequest("Invalid email or password".to_string())
        })?;

        let (role, _permissions) = self.rbac_service.get_user_permissions(user.id).await?;

        let (tokens, _user_info) = self
            .auth_provider
            .login(&email, &password, &user.password_hash, &role)
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

        info!(user_id = %user.id, "user logged in");
        Ok((user, tokens))
    }

    #[instrument(skip(self))]
    pub async fn logout(&self, refresh_token: String) -> AppResult<()> {
        info!("user logged out");
        self.session_repo.revoke(&refresh_token).await
    }

    #[instrument(skip(self))]
    pub async fn refresh_token(&self, refresh_token: String) -> AppResult<(User, Tokens)> {
        let session = self.session_repo.find_by_token(&refresh_token).await?;
        let session = session.ok_or_else(|| {
            warn!("refresh_token: invalid token");
            AppError::BadRequest("Invalid refresh token".to_string())
        })?;

        if session.expires_at < Utc::now().naive_utc() {
            self.session_repo.revoke(&refresh_token).await?;
            warn!("refresh_token: expired token revoked");
            return Err(AppError::BadRequest("Refresh token expired".to_string()));
        }

        let user = self.user_service.find_by_id(session.user_id).await?;
        let user = user.ok_or_else(|| {
            error!(user_id = %session.user_id, "refresh_token: user not found");
            AppError::Internal
        })?;

        let (role, _permissions) = self.rbac_service.get_user_permissions(user.id).await?;

        let new_access_token = self
            .auth_provider
            .generate_access_token(&user.id.to_string(), &user.email, &role)
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

        info!(user_id = %user.id, "token refreshed");
        Ok((user, tokens))
    }

    #[instrument(skip(self))]
    pub async fn get_current_user(&self, access_token: String) -> AppResult<(User, String)> {
        let user_info = self.auth_provider.introspect_token(&access_token).await?;
        let user = self.user_service.find_by_email(&user_info.email).await?;
        let user = user.ok_or_else(|| {
            warn!(email = %user_info.email, "get_current_user: user not found");
            AppError::BadRequest("User not found".to_string())
        })?;
        Ok((user, user_info.role))
    }

    #[instrument(skip(self), fields(email = %email))]
    pub async fn forgot_password(&self, email: String) -> AppResult<()> {
        let user = match self.user_service.find_by_email(&email).await {
            Ok(Some(u)) => u,
            Ok(None) => {
                info!(email = %email, "forgot_password: user not found, returning silently");
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        let token = Uuid::new_v4().to_string();
        let token_hash = format!("{:x}", Sha256::digest(token.as_bytes()));
        let expires_at = Utc::now()
            .checked_add_signed(Duration::hours(1))
            .ok_or(AppError::Internal)?
            .naive_utc();

        let reset_url = format!(
            "{}/reset-password?token={}",
            self.frontend_url.trim_end_matches('/'),
            token
        );

        self.password_reset_repo
            .create(NewPasswordReset {
                user_id: user.id,
                token_hash,
                expires_at,
            })
            .await?;

        if let Err(e) = self
            .email_provider
            .send_password_reset(&user.email, &reset_url)
            .await
        {
            warn!(user_id = %user.id, "forgot_password: failed to send email: {:?}", e);
        }

        info!(user_id = %user.id, "forgot_password: reset email sent");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn reset_password(&self, token: String, new_password: String) -> AppResult<()> {
        if new_password.len() < 8 {
            warn!("reset_password: short password");
            return Err(AppError::BadRequest(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        let token_hash = format!("{:x}", Sha256::digest(token.as_bytes()));
        let reset = self
            .password_reset_repo
            .find_by_token_hash(&token_hash)
            .await?;
        let reset = reset.ok_or_else(|| {
            warn!("reset_password: invalid token");
            AppError::BadRequest("Invalid or expired token".to_string())
        })?;

        if reset.expires_at < Utc::now().naive_utc() {
            warn!(user_id = %reset.user_id, "reset_password: expired token");
            return Err(AppError::BadRequest("Invalid or expired token".to_string()));
        }

        if reset.used_at.is_some() {
            self.session_repo.revoke_all_for_user(reset.user_id).await?;
            warn!(user_id = %reset.user_id, "reset_password: token reuse detected, sessions revoked");
            return Err(AppError::BadRequest("Token already used".to_string()));
        }

        let password_hash = bcrypt::hash(&new_password, bcrypt::DEFAULT_COST).map_err(|e| {
            error!("reset_password: bcrypt error: {:?}", e);
            AppError::Internal
        })?;

        self.user_service
            .update_password_hash(reset.user_id, password_hash)
            .await?;

        self.password_reset_repo.mark_used(reset.id).await?;
        self.session_repo.revoke_all_for_user(reset.user_id).await?;

        info!(user_id = %reset.user_id, "password reset successfully");
        Ok(())
    }
}
