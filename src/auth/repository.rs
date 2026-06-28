use async_trait::async_trait;
use diesel::prelude::*;
use tracing::instrument;
use uuid::Uuid;

use crate::auth::models::{NewPasswordReset, PasswordReset};
use crate::auth::ports::PasswordResetRepository;
use crate::schema::password_resets;
use crate::shared::db::DbPool;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct DieselPasswordResetRepository {
    pool: DbPool,
}

impl DieselPasswordResetRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PasswordResetRepository for DieselPasswordResetRepository {
    #[instrument(skip(self, reset))]
    async fn create(&self, reset: NewPasswordReset) -> AppResult<PasswordReset> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::insert_into(password_resets::table)
            .values(&reset)
            .get_result::<PasswordReset>(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })
    }

    #[instrument(skip(self))]
    async fn find_by_token_hash(&self, token_hash: &str) -> AppResult<Option<PasswordReset>> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        password_resets::table
            .filter(password_resets::token_hash.eq(token_hash))
            .first::<PasswordReset>(&mut conn)
            .optional()
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })
    }

    #[instrument(skip(self))]
    async fn mark_used(&self, id: Uuid) -> AppResult<()> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        let affected = diesel::update(password_resets::table.find(id))
            .set(password_resets::used_at.eq(chrono::Utc::now().naive_utc()))
            .execute(&mut conn)
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })?;
        if affected == 0 {
            return Err(AppError::NotFound("Password reset not found".to_string()));
        }
        Ok(())
    }
}
