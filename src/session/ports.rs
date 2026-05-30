use async_trait::async_trait;

use crate::session::models::{NewSession, Session};
use crate::shared::errors::AppResult;

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: NewSession) -> AppResult<Session>;
    async fn find_by_token(&self, token: &str) -> AppResult<Option<Session>>;
    async fn revoke(&self, token: &str) -> AppResult<()>;
}
