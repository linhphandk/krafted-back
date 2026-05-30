use async_trait::async_trait;
use diesel::prelude::*;

use crate::schema::users;
use crate::shared::db::DbPool;
use crate::shared::errors::{AppError, AppResult};
use crate::user::models::{NewUser, User};
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
}
