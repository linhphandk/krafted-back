use async_trait::async_trait;

use crate::auth::models::{Tokens, UserInfo};
use crate::shared::errors::AppResult;

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn discover_oidc(&self) -> AppResult<()>;

    async fn get_authorization_url(&self, state: &str, code_verifier: &str) -> String;

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> AppResult<(Tokens, UserInfo)>;

    async fn introspect_token(&self, token: &str) -> AppResult<UserInfo>;

    async fn refresh_token(&self, refresh_token: &str) -> AppResult<Tokens>;

    async fn revoke_token(&self, token: &str) -> AppResult<()>;
}
