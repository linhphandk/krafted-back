use krafted_back::rbac::ports::RbacRepository;
use krafted_back::rbac::repository::DieselRbacRepository;
use krafted_back::shared::db::{establish_pool, run_migrations};
use krafted_back::user::models::NewUser;
use krafted_back::user::ports::UserRepository;
use krafted_back::user::repository::DieselUserRepository;
use testcontainers::clients::Cli;
use testcontainers_modules::postgres::Postgres;

fn setup(
    docker: &Cli,
) -> (
    testcontainers::Container<'_, Postgres>,
    DieselRbacRepository,
    DieselUserRepository,
) {
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let pool = establish_pool(&db_url, 4);
    run_migrations(&pool);
    let rbac_repo = DieselRbacRepository::new(pool.clone());
    let user_repo = DieselUserRepository::new(pool);
    (container, rbac_repo, user_repo)
}

#[tokio::test]
async fn test_find_role_by_name_returns_user() {
    let docker = Cli::default();
    let (_container, repo, _) = setup(&docker);

    let role = repo.find_role_by_name("user").await.unwrap();
    assert!(role.is_some());
    assert_eq!(role.unwrap().name, "user");
}

#[tokio::test]
async fn test_find_role_by_name_returns_admin() {
    let docker = Cli::default();
    let (_container, repo, _) = setup(&docker);

    let role = repo.find_role_by_name("admin").await.unwrap();
    assert!(role.is_some());
    assert_eq!(role.unwrap().name, "admin");
}

#[tokio::test]
async fn test_find_role_by_name_returns_none() {
    let docker = Cli::default();
    let (_container, repo, _) = setup(&docker);

    let role = repo.find_role_by_name("nonexistent").await.unwrap();
    assert!(role.is_none());
}

#[tokio::test]
async fn test_assign_role() {
    let docker = Cli::default();
    let (_container, rbac_repo, user_repo) = setup(&docker);

    let user = user_repo
        .create(NewUser {
            email: "test@example.com".to_string(),
            name: "Test".to_string(),
            password_hash: String::new(),
        })
        .await
        .unwrap();

    let role = rbac_repo.find_role_by_name("user").await.unwrap().unwrap();
    let result = rbac_repo.assign_role(user.id, role.id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_user_role_ids_returns_roles() {
    let docker = Cli::default();
    let (_container, rbac_repo, user_repo) = setup(&docker);

    let user = user_repo
        .create(NewUser {
            email: "roles@example.com".to_string(),
            name: "Test".to_string(),
            password_hash: String::new(),
        })
        .await
        .unwrap();

    let role = rbac_repo.find_role_by_name("user").await.unwrap().unwrap();
    rbac_repo.assign_role(user.id, role.id).await.unwrap();

    let role_ids = rbac_repo.get_user_role_ids(user.id).await.unwrap();
    assert_eq!(role_ids.len(), 1);
    assert_eq!(role_ids[0], role.id);
}

#[tokio::test]
async fn test_get_user_role_ids_returns_empty() {
    let docker = Cli::default();
    let (_container, rbac_repo, user_repo) = setup(&docker);

    let user = user_repo
        .create(NewUser {
            email: "noroles@example.com".to_string(),
            name: "Test".to_string(),
            password_hash: String::new(),
        })
        .await
        .unwrap();

    let role_ids = rbac_repo.get_user_role_ids(user.id).await.unwrap();
    assert!(role_ids.is_empty());
}

#[tokio::test]
async fn test_get_permission_names_by_role_ids() {
    let docker = Cli::default();
    let (_container, rbac_repo, _) = setup(&docker);

    let role = rbac_repo.find_role_by_name("user").await.unwrap().unwrap();
    let permissions = rbac_repo
        .get_permission_names_by_role_ids(&[role.id])
        .await
        .unwrap();
    assert!(permissions.is_empty());
}

#[tokio::test]
async fn test_get_permission_names_by_empty_role_ids() {
    let docker = Cli::default();
    let (_container, rbac_repo, _) = setup(&docker);

    let permissions = rbac_repo
        .get_permission_names_by_role_ids(&[])
        .await
        .unwrap();
    assert!(permissions.is_empty());
}
