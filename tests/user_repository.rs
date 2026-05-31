use krafted_back::shared::db::{establish_pool, run_migrations};
use krafted_back::shared::errors::AppError;
use krafted_back::user::models::NewUser;
use krafted_back::user::ports::UserRepository;
use krafted_back::user::repository::DieselUserRepository;
use testcontainers::clients::Cli;
use testcontainers_modules::postgres::Postgres;

fn setup_repository(
    docker: &Cli,
) -> (
    testcontainers::Container<'_, Postgres>,
    DieselUserRepository,
) {
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let pool = establish_pool(&db_url, 4);
    run_migrations(&pool);
    let repo = DieselUserRepository::new(pool);
    (container, repo)
}

#[tokio::test]
async fn test_create_user() {
    let docker = Cli::default();
    let (_container, repo) = setup_repository(&docker);

    let new_user = NewUser {
        email: "test@example.com".to_string(),
        name: "Test User".to_string(),
        password_hash: String::new(),
    };
    let created = repo.create(new_user).await.unwrap();
    assert_eq!(created.email, "test@example.com");
    assert_eq!(created.name, "Test User");
}

#[tokio::test]
async fn test_create_duplicate_user_returns_bad_request() {
    let docker = Cli::default();
    let (_container, repo) = setup_repository(&docker);

    let email = format!("dup-{}@example.com", uuid::Uuid::new_v4());
    let new_user = NewUser {
        email: email.clone(),
        name: "First".to_string(),
        password_hash: String::new(),
    };
    repo.create(new_user).await.unwrap();

    let duplicate = NewUser {
        email,
        name: "Second".to_string(),
        password_hash: String::new(),
    };
    let result = repo.create(duplicate).await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}

#[tokio::test]
async fn test_find_by_email_returns_user() {
    let docker = Cli::default();
    let (_container, repo) = setup_repository(&docker);

    let email = "find@example.com".to_string();
    let new_user = NewUser {
        email: email.clone(),
        name: "Find Me".to_string(),
        password_hash: String::new(),
    };
    repo.create(new_user).await.unwrap();

    let found = repo.find_by_email(&email).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().email, email);
}

#[tokio::test]
async fn test_find_by_email_returns_none() {
    let docker = Cli::default();
    let (_container, repo) = setup_repository(&docker);

    let found = repo.find_by_email("nope@example.com").await.unwrap();
    assert!(found.is_none());
}
