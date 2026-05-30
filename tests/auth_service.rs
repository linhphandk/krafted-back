use async_trait::async_trait;
use chrono::NaiveDateTime;
use krafted_back::auth::models::{Tokens, UserInfo};
use krafted_back::auth::ports::AuthProvider;
use krafted_back::auth::service::AuthService;
use krafted_back::shared::errors::{AppError, AppResult};
use krafted_back::user::models::{NewUser, User};
use krafted_back::user::ports::UserRepository;
use mockall::mock;
use uuid::Uuid;

mock! {
    pub MockAuthProvider {}

    #[async_trait]
    impl AuthProvider for MockAuthProvider {
        async fn register(&self, email: &str, name: &str, password: &str) -> AppResult<UserInfo>;
        async fn login(&self, email: &str, password: &str) -> AppResult<(Tokens, UserInfo)>;
        async fn introspect_token(&self, token: &str) -> AppResult<UserInfo>;
        async fn refresh_token(&self, refresh_token: &str) -> AppResult<Tokens>;
        async fn revoke_token(&self, token: &str) -> AppResult<()>;
    }
}

mock! {
    pub MockUserRepo {}

    #[async_trait]
    impl UserRepository for MockUserRepo {
        async fn create(&self, user: NewUser) -> AppResult<User>;
    }
}

fn fake_user() -> User {
    User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        name: "Test".to_string(),
        created_at: NaiveDateTime::default(),
        updated_at: NaiveDateTime::default(),
        password_hash: String::new(),
    }
}

fn fake_user_info() -> UserInfo {
    UserInfo {
        sub: "1".to_string(),
        email: "test@example.com".to_string(),
        name: "Test".to_string(),
    }
}

#[tokio::test]
async fn test_register_success() {
    let mut mock_auth = MockMockAuthProvider::new();
    mock_auth
        .expect_register()
        .returning(|_, _, _| Ok(fake_user_info()));

    let mut mock_repo = MockMockUserRepo::new();
    mock_repo.expect_create().returning(|_| Ok(fake_user()));

    let service = AuthService::new(mock_auth, mock_repo);
    let result = service
        .register(
            "test@example.com".to_string(),
            "Test".to_string(),
            "password123".to_string(),
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_register_empty_email() {
    let mock_auth = MockMockAuthProvider::new();
    let mock_repo = MockMockUserRepo::new();
    let service = AuthService::new(mock_auth, mock_repo);
    let result = service
        .register(
            "".to_string(),
            "Test".to_string(),
            "password123".to_string(),
        )
        .await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_register_empty_name() {
    let mock_auth = MockMockAuthProvider::new();
    let mock_repo = MockMockUserRepo::new();
    let service = AuthService::new(mock_auth, mock_repo);
    let result = service
        .register(
            "test@example.com".to_string(),
            "".to_string(),
            "password123".to_string(),
        )
        .await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_register_short_password() {
    let mock_auth = MockMockAuthProvider::new();
    let mock_repo = MockMockUserRepo::new();
    let service = AuthService::new(mock_auth, mock_repo);
    let result = service
        .register(
            "test@example.com".to_string(),
            "Test".to_string(),
            "short".to_string(),
        )
        .await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_register_delegates_to_auth_provider() {
    let mut mock_auth = MockMockAuthProvider::new();
    mock_auth
        .expect_register()
        .withf(|email, name, _| email == "test@example.com" && name == "Test")
        .times(1)
        .returning(|_, _, _| Ok(fake_user_info()));

    let mut mock_repo = MockMockUserRepo::new();
    mock_repo.expect_create().returning(|_| Ok(fake_user()));

    let service = AuthService::new(mock_auth, mock_repo);
    let result = service
        .register(
            "test@example.com".to_string(),
            "Test".to_string(),
            "password123".to_string(),
        )
        .await;
    assert!(result.is_ok());
}
