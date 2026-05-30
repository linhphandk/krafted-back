use async_trait::async_trait;

use crate::auth::models::{Tokens, UserInfo};
use crate::auth::ports::AuthProvider;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct MockAuthProvider;

#[async_trait]
impl AuthProvider for MockAuthProvider {
    async fn register(&self, _email: &str, _name: &str, _password: &str) -> AppResult<UserInfo> {
        Err(AppError::NotImplemented)
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
