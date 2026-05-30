use async_trait::async_trait;
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::users;
use crate::shared::db::DbPool;
use crate::shared::errors::{AppError, AppResult};
use crate::user::models::{NewUser, UpdateUser, User};
use crate::user::ports::UserRepository;

pub struct DieselUserRepository {
    pool: DbPool,
}

impl DieselUserRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for DieselUserRepository {
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        let mut conn = self.pool.get().map_err(|e| {
            AppError::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(e.to_string()),
            ))
        })?;
        Ok(users::table.find(id).first(&mut conn).optional()?)
    }

    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let mut conn = self.pool.get().map_err(|e| {
            AppError::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(e.to_string()),
            ))
        })?;
        Ok(users::table
            .filter(users::email.eq(email))
            .first(&mut conn)
            .optional()?)
    }

    async fn create(&self, new_user: NewUser) -> AppResult<User> {
        let mut conn = self.pool.get().map_err(|e| {
            AppError::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(e.to_string()),
            ))
        })?;
        Ok(diesel::insert_into(users::table)
            .values(&new_user)
            .get_result(&mut conn)?)
    }

    async fn update(&self, id: Uuid, data: UpdateUser) -> AppResult<User> {
        let mut conn = self.pool.get().map_err(|e| {
            AppError::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(e.to_string()),
            ))
        })?;
        Ok(diesel::update(users::table.find(id))
            .set(&data)
            .get_result(&mut conn)?)
    }

    async fn delete(&self, id: Uuid) -> AppResult<()> {
        let mut conn = self.pool.get().map_err(|e| {
            AppError::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(e.to_string()),
            ))
        })?;
        diesel::delete(users::table.find(id)).execute(&mut conn)?;
        Ok(())
    }
}
