use async_trait::async_trait;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::schema::users;
use crate::shared::db::DbPool;
use crate::shared::errors::{AppError, AppResult};
use crate::user::models::{NewUser, UpdateUser, User};
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
    #[instrument(skip(self, new_user), fields(email = %new_user.email))]
    async fn create(&self, new_user: NewUser) -> AppResult<User> {
        debug!("create user in db");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::insert_into(users::table)
            .values(&new_user)
            .get_result::<User>(&mut conn)
            .map_err(|e| map_diesel_error(e, "Email"))
    }

    #[instrument(skip(self), fields(email = %email))]
    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        debug!(email = %email, "find_by_email in db");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        users::table
            .filter(users::email.eq(email))
            .first::<User>(&mut conn)
            .optional()
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })
    }

    #[instrument(skip(self), fields(user_id = %id))]
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        debug!(user_id = %id, "find_by_id in db");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        users::table
            .filter(users::id.eq(id))
            .first::<User>(&mut conn)
            .optional()
            .map_err(|e| {
                tracing::error!("Database error: {:?}", e);
                AppError::Internal
            })
    }

    #[instrument(skip(self), fields(user_id = %id))]
    async fn update(&self, id: Uuid, data: UpdateUser) -> AppResult<User> {
        debug!(user_id = %id, "update user");
        let mut conn = self.pool.get().map_err(|e| {
            tracing::error!("Connection pool error: {:?}", e);
            AppError::Internal
        })?;
        diesel::update(users::table.find(id))
            .set(&data)
            .get_result::<User>(&mut conn)
            .map_err(|e| map_diesel_error(e, "User"))
    }
}
