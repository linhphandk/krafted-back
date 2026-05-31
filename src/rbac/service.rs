use std::sync::Arc;

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

    pub async fn assign_default_role(&self, user_id: Uuid) -> AppResult<()> {
        let role = self.repo.find_role_by_name("user").await?;
        let role = role.ok_or(AppError::Internal)?;
        self.repo.assign_role(user_id, role.id).await
    }
}
