use async_trait::async_trait;
use diesel::prelude::*;
use uuid::Uuid;

use crate::rbac::models::{Role, UserRole};
use crate::rbac::ports::RbacRepository;
use crate::schema::{permissions, role_permissions, roles, user_roles};
use crate::shared::db::DbPool;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct DieselRbacRepository {
    pool: DbPool,
}

impl DieselRbacRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RbacRepository for DieselRbacRepository {
    async fn find_role_by_name(&self, name: &str) -> AppResult<Option<Role>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        roles::table
            .filter(roles::name.eq(name))
            .first::<Role>(&mut conn)
            .optional()
            .map_err(|e| {
                tracing::error!("Failed to find role by name: {:?}", e);
                AppError::Internal
            })
    }

    async fn assign_role(&self, user_id: Uuid, role_id: Uuid) -> AppResult<()> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::insert_into(user_roles::table)
            .values(UserRole {
                user_id,
                role_id,
                created_at: chrono::Utc::now().naive_utc(),
            })
            .execute(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to assign role: {:?}", e);
                AppError::Internal
            })?;
        Ok(())
    }

    async fn get_user_role_ids(&self, user_id: Uuid) -> AppResult<Vec<Uuid>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        user_roles::table
            .filter(user_roles::user_id.eq(user_id))
            .select(user_roles::role_id)
            .load::<Uuid>(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to get user role ids: {:?}", e);
                AppError::Internal
            })
    }

    async fn get_permission_names_by_role_ids(&self, role_ids: &[Uuid]) -> AppResult<Vec<String>> {
        if role_ids.is_empty() {
            return Ok(vec![]);
        }
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        permissions::table
            .filter(
                permissions::id.eq_any(
                    role_permissions::table
                        .filter(role_permissions::role_id.eq_any(role_ids))
                        .select(role_permissions::permission_id),
                ),
            )
            .select(permissions::name)
            .load::<String>(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to get permission names: {:?}", e);
                AppError::Internal
            })
    }
}
