use async_trait::async_trait;

use crate::auth::models::{Tokens, UserInfo};
use crate::auth::ports::AuthProvider;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct LocalAuthProvider;

impl LocalAuthProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AuthProvider for LocalAuthProvider {
    async fn register(&self, email: &str, name: &str, password: &str) -> AppResult<UserInfo> {
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
            tracing::error!("Password hashing error: {:?}", e);
            AppError::Internal
        })?;

        Ok(UserInfo {
            sub: uuid::Uuid::new_v4().to_string(),
            email: email.to_string(),
            name: name.to_string(),
            password_hash,
        })
    }

    async fn login(&self, _email: &str, _password: &str) -> AppResult<(Tokens, UserInfo)> {
        Err(AppError::NotImplemented)
    }

    async fn introspect_token(&self, _token: &str) -> AppResult<UserInfo> {
        Err(AppError::NotImplemented)
    }

    async fn refresh_token(&self, _refresh_token: &str) -> AppResult<Tokens> {
        Err(AppError::NotImplemented)
    }

    async fn revoke_token(&self, _token: &str) -> AppResult<()> {
        Err(AppError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_hashes_password() {
        let provider = LocalAuthProvider::new();
        let result = provider
            .register("test@example.com", "Test", "password123")
            .await;

        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.password_hash.starts_with("$2b$"));
        assert_eq!(info.email, "test@example.com");
        assert_eq!(info.name, "Test");
        assert!(!info.sub.is_empty());
    }
}
