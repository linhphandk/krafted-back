use async_trait::async_trait;
use chrono::NaiveDateTime;
use krafted_back::shared::errors::{AppError, AppResult};
use krafted_back::user::models::{NewUser, User};
use krafted_back::user::ports::UserRepository;
use krafted_back::user::service::UserService;
use mockall::mock;
use uuid::Uuid;

mock! {
    pub MockUserRepo {}

    #[async_trait]
    impl UserRepository for MockUserRepo {
        async fn create(&self, user: NewUser) -> AppResult<User>;
        async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
        async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>>;
        async fn update(&self, id: Uuid, data: krafted_back::user::models::UpdateUser) -> AppResult<User>;
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

#[tokio::test]
async fn test_create_user_success() {
    let mut mock_repo = MockMockUserRepo::new();
    mock_repo.expect_create().returning(|_| Ok(fake_user()));

    let service = UserService::new(mock_repo);
    let new_user = NewUser {
        email: "test@example.com".to_string(),
        name: "Test".to_string(),
        password_hash: "hash".to_string(),
    };
    let result = service.create(new_user).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_user_empty_email() {
    let mock_repo = MockMockUserRepo::new();
    let service = UserService::new(mock_repo);
    let new_user = NewUser {
        email: "".to_string(),
        name: "Test".to_string(),
        password_hash: "hash".to_string(),
    };
    let result = service.create(new_user).await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_create_user_empty_name() {
    let mock_repo = MockMockUserRepo::new();
    let service = UserService::new(mock_repo);
    let new_user = NewUser {
        email: "test@example.com".to_string(),
        name: "".to_string(),
        password_hash: "hash".to_string(),
    };
    let result = service.create(new_user).await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_create_user_repo_error() {
    let mut mock_repo = MockMockUserRepo::new();
    mock_repo
        .expect_create()
        .returning(|_| Err(AppError::BadRequest("Email already exists".to_string())));

    let service = UserService::new(mock_repo);
    let new_user = NewUser {
        email: "dup@example.com".to_string(),
        name: "Test".to_string(),
        password_hash: "hash".to_string(),
    };
    let result = service.create(new_user).await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}
