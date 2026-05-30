use async_trait::async_trait;

use crate::auth::models::{Tokens, UserInfo};
use crate::shared::errors::AppResult;

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn register(&self, email: &str, name: &str, password: &str) -> AppResult<UserInfo>;

    async fn login(&self, email: &str, password: &str) -> AppResult<(Tokens, UserInfo)>;

    async fn introspect_token(&self, token: &str) -> AppResult<UserInfo>;

    async fn refresh_token(&self, refresh_token: &str) -> AppResult<Tokens>;

    async fn revoke_token(&self, token: &str) -> AppResult<()>;
}
