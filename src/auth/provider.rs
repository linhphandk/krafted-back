use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;

use crate::auth::models::{Tokens, UserInfo};
use crate::auth::ports::AuthProvider;
use crate::shared::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct LocalAuthProvider {
    jwt_secret: String,
    jwt_expiry_minutes: u64,
}

#[derive(Serialize)]
struct Claims {
    sub: String,
    email: String,
    exp: usize,
}

impl LocalAuthProvider {
    pub fn new(jwt_secret: String, jwt_expiry_minutes: u64) -> Self {
        Self {
            jwt_secret,
            jwt_expiry_minutes,
        }
    }

    fn generate_access_token(&self, user_id: &str, email: &str) -> AppResult<String> {
        let exp = Utc::now()
            .checked_add_signed(chrono::Duration::minutes(self.jwt_expiry_minutes as i64))
            .ok_or(AppError::Internal)?
            .timestamp() as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            exp,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            tracing::error!("JWT encoding error: {:?}", e);
            AppError::Internal
        })
    }
}

#[async_trait]
impl AuthProvider for LocalAuthProvider {
    async fn register(
        &self,
        email: &str,
        name: &str,
        password: &str,
    ) -> AppResult<(UserInfo, Tokens)> {
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
            tracing::error!("Password hashing error: {:?}", e);
            AppError::Internal
        })?;

        let user_id = uuid::Uuid::new_v4().to_string();
        let access_token = self.generate_access_token(&user_id, email)?;

        let user_info = UserInfo {
            sub: user_id,
            email: email.to_string(),
            name: name.to_string(),
            password_hash,
        };

        let tokens = Tokens {
            access_token,
            refresh_token: String::new(),
            id_token: String::new(),
            expires_in: self.jwt_expiry_minutes * 60,
        };

        Ok((user_info, tokens))
    }

    async fn login(&self, email: &str, password: &str, password_hash: &str) -> AppResult<(Tokens, UserInfo)> {
        let valid = bcrypt::verify(password, password_hash).map_err(|e| {
            tracing::error!("Password verification error: {:?}", e);
            AppError::Internal
        })?;

        if !valid {
            return Err(AppError::BadRequest("Invalid email or password".to_string()));
        }

        let access_token = self.generate_access_token(email, email)?;

        let tokens = Tokens {
            access_token,
            refresh_token: String::new(),
            id_token: String::new(),
            expires_in: self.jwt_expiry_minutes * 60,
        };

        let user_info = UserInfo {
            sub: email.to_string(),
            email: email.to_string(),
            name: String::new(),
            password_hash: password_hash.to_string(),
        };

        Ok((tokens, user_info))
    }

    async fn introspect_token(&self, _token: &str) -> AppResult<UserInfo> {
        Err(AppError::NotImplemented)
    }

    async fn refresh_token(&self, _refresh_token: &str) -> AppResult<Tokens> {
        Err(AppError::NotImplemented)
    }

    async fn revoke_token(&self, _token: &str) -> AppResult<()> {
        Err(AppError::NotImplemented)
    }

    async fn generate_access_token(&self, user_id: &str, email: &str) -> AppResult<String> {
        self.generate_access_token(user_id, email)
    }

    fn token_expiry_seconds(&self) -> u64 {
        self.jwt_expiry_minutes * 60
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_hashes_password_and_returns_tokens() {
        let provider = LocalAuthProvider::new("test-secret".to_string(), 15);
        let result = provider
            .register("test@example.com", "Test", "password123")
            .await;

        assert!(result.is_ok());
        let (info, tokens) = result.unwrap();
        assert!(info.password_hash.starts_with("$2b$"));
        assert_eq!(info.email, "test@example.com");
        assert!(!tokens.access_token.is_empty());
    }
}
