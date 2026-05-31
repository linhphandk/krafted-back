use krafted_back::auth::provider::LocalAuthProvider;
use krafted_back::auth::service::AuthService;
use krafted_back::rbac::ports::RbacRepository;
use krafted_back::rbac::repository::DieselRbacRepository;
use krafted_back::rbac::service::RbacService;
use krafted_back::session::repository::DieselSessionRepository;
use krafted_back::shared::db::{establish_pool, run_migrations};
use krafted_back::user::repository::DieselUserRepository;
use std::sync::Arc;
use testcontainers::clients::Cli;
use testcontainers_modules::postgres::Postgres;

fn setup_auth_service(
    docker: &Cli,
) -> (
    testcontainers::Container<'_, Postgres>,
    AuthService<LocalAuthProvider, DieselUserRepository, DieselSessionRepository>,
    DieselRbacRepository,
) {
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let pool = establish_pool(&db_url, 4);
    run_migrations(&pool);

    let auth_provider = LocalAuthProvider::new("test-secret".to_string(), 15);
    let user_repo = DieselUserRepository::new(pool.clone());
    let session_repo = DieselSessionRepository::new(pool.clone());
    let rbac_repo = DieselRbacRepository::new(pool.clone());
    let rbac_service = Arc::new(RbacService::new(Arc::new(rbac_repo.clone())));
    let service = AuthService::new(auth_provider, user_repo, session_repo, 7, rbac_service);

    (container, service, rbac_repo)
}

#[tokio::test]
async fn test_register_creates_user_in_db_with_hashed_password_and_tokens() {
    let docker = Cli::default();
    let (_container, service, rbac_repo) = setup_auth_service(&docker);

    let (user, tokens) = service
        .register(
            "test@example.com".to_string(),
            "Test User".to_string(),
            "password123".to_string(),
        )
        .await
        .unwrap();

    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.name, "Test User");
    assert!(user.password_hash.starts_with("$2b$"));
    assert!(!tokens.access_token.is_empty());
    assert_eq!(tokens.expires_in, 900);

    let role_ids = rbac_repo.get_user_role_ids(user.id).await.unwrap();
    assert_eq!(role_ids.len(), 1);

    let user_role = rbac_repo.find_role_by_name("user").await.unwrap().unwrap();
    assert_eq!(role_ids[0], user_role.id);
}

#[tokio::test]
async fn test_register_duplicate_email_fails() {
    let docker = Cli::default();
    let (_container, service, _) = setup_auth_service(&docker);

    service
        .register(
            "dup@example.com".to_string(),
            "First".to_string(),
            "password123".to_string(),
        )
        .await
        .unwrap();

    let result = service
        .register(
            "dup@example.com".to_string(),
            "Second".to_string(),
            "password123".to_string(),
        )
        .await;

    assert!(result.is_err());
}
