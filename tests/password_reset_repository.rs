use diesel::prelude::*;
use krafted_back::auth::models::NewPasswordReset;
use krafted_back::auth::ports::PasswordResetRepository;
use krafted_back::auth::repository::DieselPasswordResetRepository;
use krafted_back::schema::users;
use krafted_back::shared::db::{establish_pool, run_migrations};
use krafted_back::shared::errors::AppError;
use krafted_back::user::models::NewUser;
use testcontainers::clients::Cli;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

fn setup(
    docker: &Cli,
) -> (
    testcontainers::Container<'_, Postgres>,
    DieselPasswordResetRepository,
    Uuid,
) {
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let pool = establish_pool(&db_url, 4);
    run_migrations(&pool);

    let mut conn = pool.get().unwrap();
    let user_id = diesel::insert_into(users::table)
        .values(&NewUser {
            email: format!("user-{}@example.com", Uuid::new_v4()),
            name: "Test User".to_string(),
            password_hash: String::new(),
        })
        .returning(users::id)
        .get_result::<Uuid>(&mut conn)
        .unwrap();
    drop(conn);

    let repo = DieselPasswordResetRepository::new(pool);
    (container, repo, user_id)
}

#[tokio::test]
async fn test_create_password_reset() {
    let docker = Cli::default();
    let (_container, repo, user_id) = setup(&docker);

    let token_hash = "a".repeat(64);
    let expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::hours(1);
    let reset = repo
        .create(NewPasswordReset {
            user_id,
            token_hash: token_hash.clone(),
            expires_at,
        })
        .await
        .unwrap();

    assert_eq!(reset.user_id, user_id);
    assert_eq!(reset.token_hash, token_hash);
    assert!(reset.used_at.is_none());
}

#[tokio::test]
async fn test_find_by_token_hash_found() {
    let docker = Cli::default();
    let (_container, repo, user_id) = setup(&docker);

    let token_hash = "b".repeat(64);
    let expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::hours(1);
    let created = repo
        .create(NewPasswordReset {
            user_id,
            token_hash: token_hash.clone(),
            expires_at,
        })
        .await
        .unwrap();

    let found = repo.find_by_token_hash(&token_hash).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, created.id);
    assert_eq!(found.user_id, user_id);
    assert!(found.used_at.is_none());
}

#[tokio::test]
async fn test_find_by_token_hash_not_found() {
    let docker = Cli::default();
    let (_container, repo, _user_id) = setup(&docker);

    let found = repo.find_by_token_hash("nonexistent_hash").await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_mark_used() {
    let docker = Cli::default();
    let (_container, repo, user_id) = setup(&docker);

    let token_hash = "c".repeat(64);
    let expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::hours(1);
    let created = repo
        .create(NewPasswordReset {
            user_id,
            token_hash,
            expires_at,
        })
        .await
        .unwrap();

    repo.mark_used(created.id).await.unwrap();

    let found = repo
        .find_by_token_hash(&created.token_hash)
        .await
        .unwrap()
        .unwrap();
    assert!(found.used_at.is_some());
}

#[tokio::test]
async fn test_mark_used_not_found() {
    let docker = Cli::default();
    let (_container, repo, _user_id) = setup(&docker);

    let result = repo.mark_used(Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}
