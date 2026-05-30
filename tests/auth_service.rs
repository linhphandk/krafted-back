use async_trait::async_trait;
use chrono::NaiveDateTime;
use krafted_back::auth::service::AuthService;
use krafted_back::shared::errors::{AppError, AppResult};
use krafted_back::user::models::{NewUser, User};
use krafted_back::user::ports::UserRepository;
use mockall::mock;
use uuid::Uuid;

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
        password_hash: "$2b$12$hashed".to_string(),
    }
}

#[tokio::test]
async fn test_register_success() {
    let mut mock_repo = MockMockUserRepo::new();
    mock_repo.expect_create().returning(|_| Ok(fake_user()));

    let service = AuthService::new(mock_repo);
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
    let mock_repo = MockMockUserRepo::new();
    let service = AuthService::new(mock_repo);
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
    let mock_repo = MockMockUserRepo::new();
    let service = AuthService::new(mock_repo);
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
    let mock_repo = MockMockUserRepo::new();
    let service = AuthService::new(mock_repo);
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
async fn test_register_password_is_hashed() {
    let mut mock_repo = MockMockUserRepo::new();
    mock_repo.expect_create().returning(|new_user| {
        assert!(!new_user.password_hash.is_empty());
        assert!(new_user.password_hash.starts_with("$2b$"));
        Ok(fake_user())
    });

    let service = AuthService::new(mock_repo);
    let result = service
        .register(
            "test@example.com".to_string(),
            "Test".to_string(),
            "password123".to_string(),
        )
        .await;
    assert!(result.is_ok());
}
