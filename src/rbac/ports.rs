use async_trait::async_trait;
use uuid::Uuid;

use crate::rbac::models::Role;
use crate::shared::errors::AppResult;

#[async_trait]
pub trait RbacRepository: Send + Sync {
    async fn find_role_by_name(&self, name: &str) -> AppResult<Option<Role>>;
    async fn assign_role(&self, user_id: Uuid, role_id: Uuid) -> AppResult<()>;
    async fn get_user_role_ids(&self, user_id: Uuid) -> AppResult<Vec<Uuid>>;
    async fn get_permission_names_by_role_ids(&self, role_ids: &[Uuid]) -> AppResult<Vec<String>>;
}
