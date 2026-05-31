use async_trait::async_trait;
use diesel::prelude::*;
use tracing::{debug, instrument};

use crate::schema::sessions;
use crate::session::models::{NewSession, Session};
use crate::session::ports::SessionRepository;
use crate::shared::db::DbPool;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct DieselSessionRepository {
    pool: DbPool,
}

impl DieselSessionRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for DieselSessionRepository {
    #[instrument(skip(self, session), fields(user_id = %session.user_id))]
    async fn create(&self, session: NewSession) -> AppResult<Session> {
        debug!("create session");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::insert_into(sessions::table)
            .values(&session)
            .get_result::<Session>(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to create session: {:?}", e);
                AppError::Internal
            })
    }

    #[instrument(skip(self))]
    async fn find_by_token(&self, token: &str) -> AppResult<Option<Session>> {
        debug!("find_by_token");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        sessions::table
            .filter(sessions::refresh_token.eq(token))
            .first::<Session>(&mut conn)
            .optional()
            .map_err(|e| {
                tracing::error!("Failed to find session by token: {:?}", e);
                AppError::Internal
            })
    }

    #[instrument(skip(self))]
    async fn revoke(&self, token: &str) -> AppResult<()> {
        debug!("revoke session");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::delete(sessions::table.filter(sessions::refresh_token.eq(token)))
            .execute(&mut conn)
            .map_err(|e| {
                tracing::error!("Failed to revoke session: {:?}", e);
                AppError::Internal
            })?;
        Ok(())
    }
}
