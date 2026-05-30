use async_trait::async_trait;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

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

fn map_diesel_error(e: DieselError, context: &str) -> AppError {
    match e {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            AppError::BadRequest(format!("{} already exists", context))
        }
        DieselError::NotFound => AppError::NotFound(format!("{} not found", context)),
        _ => AppError::Database(e),
    }
}

#[async_trait]
impl UserRepository for DieselUserRepository {
    async fn create(&self, new_user: NewUser) -> AppResult<User> {
        let mut conn = self.pool.get().map_err(|e| {
            AppError::Database(DieselError::DatabaseError(
                DatabaseErrorKind::Unknown,
                Box::new(e.to_string()),
            ))
        })?;
        diesel::insert_into(users::table)
            .values(&new_user)
            .get_result(&mut conn)
            .map_err(|e| map_diesel_error(e, "Email"))
    }
}
