use async_trait::async_trait;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;

use crate::schema::users;
use crate::shared::db::DbPool;
use crate::shared::errors::{AppError, AppResult};
use crate::user::models::{NewUser, User};
use crate::user::ports::UserRepository;

#[derive(Clone)]
pub struct DieselUserRepository {
    pool: DbPool,
}

impl DieselUserRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

fn map_diesel_error(e: diesel::result::Error, context: &str) -> AppError {
    match e {
        diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            AppError::BadRequest(format!("{} already exists", context))
        }
        _ => {
            tracing::error!("Database error: {:?}", e);
            AppError::Internal
        }
    }
}

#[async_trait]
impl UserRepository for DieselUserRepository {
    async fn create(&self, new_user: NewUser) -> AppResult<User> {
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::insert_into(users::table)
            .values(&new_user)
            .get_result(&mut conn)
            .map_err(|e| map_diesel_error(e, "Email"))
    }
}
