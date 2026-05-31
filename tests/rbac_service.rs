use async_trait::async_trait;
use krafted_back::rbac::models::Role;
use krafted_back::rbac::ports::RbacRepository;
use krafted_back::rbac::service::RbacService;
use krafted_back::shared::errors::{AppError, AppResult};
use mockall::mock;
use std::sync::Arc;
use uuid::Uuid;

mock! {
    pub MockRbacRepo {}

    #[async_trait]
    impl RbacRepository for MockRbacRepo {
        async fn find_role_by_name(&self, name: &str) -> AppResult<Option<Role>>;
        async fn assign_role(&self, user_id: Uuid, role_id: Uuid) -> AppResult<()>;
        async fn get_user_role_ids(&self, user_id: Uuid) -> AppResult<Vec<Uuid>>;
        async fn get_permission_names_by_role_ids(&self, role_ids: &[Uuid]) -> AppResult<Vec<String>>;
    }
}

fn fake_role() -> Role {
    Role {
        id: Uuid::new_v4(),
        name: "user".to_string(),
        description: Some("Default role".to_string()),
        created_at: chrono::Utc::now().naive_utc(),
    }
}

#[tokio::test]
async fn test_assign_default_role_success() {
    let mut mock_repo = MockMockRbacRepo::new();

    mock_repo
        .expect_find_role_by_name()
        .with(mockall::predicate::eq("user"))
        .returning(|_| Ok(Some(fake_role())));

    mock_repo.expect_assign_role().returning(|_, _| Ok(()));

    let service = RbacService::new(Arc::new(mock_repo));
    let result = service.assign_default_role(Uuid::new_v4()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_assign_default_role_not_found() {
    let mut mock_repo = MockMockRbacRepo::new();
    mock_repo
        .expect_find_role_by_name()
        .with(mockall::predicate::eq("user"))
        .returning(|_| Ok(None));

    let service = RbacService::new(Arc::new(mock_repo));
    let result = service.assign_default_role(Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn test_assign_default_role_assign_fails() {
    let mut mock_repo = MockMockRbacRepo::new();
    mock_repo
        .expect_find_role_by_name()
        .with(mockall::predicate::eq("user"))
        .returning(|_| Ok(Some(fake_role())));

    mock_repo
        .expect_assign_role()
        .returning(|_, _| Err(AppError::Internal));

    let service = RbacService::new(Arc::new(mock_repo));
    let result = service.assign_default_role(Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::Internal)));
}
