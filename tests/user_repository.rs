use diesel::prelude::*;
use krafted_back::shared::db::establish_pool;
use krafted_back::shared::errors::AppError;
use krafted_back::user::models::NewUser;
use krafted_back::user::ports::UserRepository;
use krafted_back::user::repository::DieselUserRepository;

fn setup_repository() -> DieselUserRepository {
    let db_url = "postgres://postgres:example@localhost:5432/postgres";
    let pool = establish_pool(db_url, 4);
    let mut conn = pool.get().expect("Failed to get connection");
    diesel::sql_query("DELETE FROM users")
        .execute(&mut conn)
        .ok();
    DieselUserRepository::new(pool)
}

#[tokio::test]
async fn test_create_user() {
    let repo = setup_repository();
    let new_user = NewUser {
        email: "test@example.com".to_string(),
        name: "Test User".to_string(),
    };
    let created = repo.create(new_user).await.unwrap();
    assert_eq!(created.email, "test@example.com");
    assert_eq!(created.name, "Test User");
}

#[tokio::test]
async fn test_create_duplicate_user_returns_bad_request() {
    let repo = setup_repository();
    let new_user = NewUser {
        email: "duplicate@example.com".to_string(),
        name: "First".to_string(),
    };
    repo.create(new_user).await.unwrap();

    let duplicate = NewUser {
        email: "duplicate@example.com".to_string(),
        name: "Second".to_string(),
    };
    let result = repo.create(duplicate).await;
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}
