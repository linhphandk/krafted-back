use chrono::{Duration, Utc};
use uuid::Uuid;

use krafted_back::session::models::NewSession;
use krafted_back::session::ports::SessionRepository;
use krafted_back::session::repository::DieselSessionRepository;
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
    DieselUserRepository,
    DieselSessionRepository,
) {
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let pool = establish_pool(&db_url, 4);
    run_migrations(&pool);
    let user_repo = DieselUserRepository::new(pool.clone());
    let session_repo = DieselSessionRepository::new(pool);
    (container, user_repo, session_repo)
}

#[tokio::test]
async fn test_create_session() {
    let docker = Cli::default();
    let (_container, user_repo, session_repo) = setup(&docker);

    let user = user_repo
        .create(NewUser {
            email: "test@example.com".to_string(),
            name: "Test".to_string(),
            password_hash: String::new(),
        })
        .await
        .unwrap();

    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(7))
        .unwrap()
        .naive_utc();

    let session = session_repo
        .create(NewSession {
            user_id: user.id,
            refresh_token: "test-refresh-token".to_string(),
            expires_at,
        })
        .await
        .unwrap();

    assert_eq!(session.user_id, user.id);
    assert_eq!(session.refresh_token, "test-refresh-token");
}

#[tokio::test]
async fn test_find_session_by_token() {
    let docker = Cli::default();
    let (_container, user_repo, session_repo) = setup(&docker);

    let user = user_repo
        .create(NewUser {
            email: "find@example.com".to_string(),
            name: "Find".to_string(),
            password_hash: String::new(),
        })
        .await
        .unwrap();

    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(7))
        .unwrap()
        .naive_utc();

    session_repo
        .create(NewSession {
            user_id: user.id,
            refresh_token: "find-me-token".to_string(),
            expires_at,
        })
        .await
        .unwrap();

    let found = session_repo.find_by_token("find-me-token").await.unwrap();
    assert!(found.is_some());
    let session = found.unwrap();
    assert_eq!(session.user_id, user.id);
}

#[tokio::test]
async fn test_find_nonexistent_session_returns_none() {
    let docker = Cli::default();
    let (_container, _user_repo, session_repo) = setup(&docker);

    let found = session_repo.find_by_token("does-not-exist").await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_revoke_session() {
    let docker = Cli::default();
    let (_container, user_repo, session_repo) = setup(&docker);

    let user = user_repo
        .create(NewUser {
            email: "revoke@example.com".to_string(),
            name: "Revoke".to_string(),
            password_hash: String::new(),
        })
        .await
        .unwrap();

    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(7))
        .unwrap()
        .naive_utc();

    session_repo
        .create(NewSession {
            user_id: user.id,
            refresh_token: "revoke-me".to_string(),
            expires_at,
        })
        .await
        .unwrap();

    session_repo.revoke("revoke-me").await.unwrap();

    let found = session_repo.find_by_token("revoke-me").await.unwrap();
    assert!(found.is_none());
}
