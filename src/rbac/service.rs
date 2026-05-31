use std::sync::Arc;
use tracing::{debug, error, instrument, warn};
use uuid::Uuid;

use crate::rbac::ports::RbacRepository;
use crate::shared::errors::{AppError, AppResult};

pub struct RbacService {
    repo: Arc<dyn RbacRepository>,
}

impl RbacService {
    pub fn new(repo: Arc<dyn RbacRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self), fields(user_id = %user_id))]
    pub async fn assign_default_role(&self, user_id: Uuid) -> AppResult<()> {
        debug!("assign_default_role");
        let role = self.repo.find_role_by_name("user").await?;
        let role = role.ok_or_else(|| {
            error!("default role 'user' not found");
            AppError::Internal
        })?;
        self.repo.assign_role(user_id, role.id).await
    }

    #[instrument(skip(self), fields(user_id = %user_id))]
    pub async fn get_user_permissions(&self, user_id: Uuid) -> AppResult<(String, Vec<String>)> {
        debug!("get_user_permissions");
        let role_ids = self.repo.get_user_role_ids(user_id).await?;
        if role_ids.is_empty() {
            warn!(user_id = %user_id, "user has no roles");
            return Err(AppError::Internal);
        }
        let permissions = self
            .repo
            .get_permission_names_by_role_ids(&role_ids)
            .await?;
        let role = self.repo.find_role_by_name("user").await?.ok_or_else(|| {
            error!("default role 'user' not found");
            AppError::Internal
        })?;
        debug!(role = %role.name, perms = ?permissions, "get_user_permissions ok");
        Ok((role.name, permissions))
    }
}
