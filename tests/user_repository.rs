use diesel::prelude::*;
use krafted_back::shared::db::establish_pool;
use krafted_back::user::models::{NewUser, UpdateUser};
use krafted_back::user::ports::UserRepository;
use krafted_back::user::repository::DieselUserRepository;
use uuid::Uuid;

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
async fn test_create_and_find_by_id() {
    let repo = setup_repository();
    let new_user = NewUser {
        email: "test1@example.com".to_string(),
        name: "Test User 1".to_string(),
    };
    let created = repo.create(new_user).await.unwrap();
    assert_eq!(created.email, "test1@example.com");

    let found = repo.find_by_id(created.id).await.unwrap().unwrap();
    assert_eq!(found.id, created.id);
}

#[tokio::test]
async fn test_find_by_email() {
    let repo = setup_repository();
    let new_user = NewUser {
        email: "test2@example.com".to_string(),
        name: "Test User 2".to_string(),
    };
    repo.create(new_user).await.unwrap();

    let found = repo
        .find_by_email("test2@example.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.email, "test2@example.com");
}

#[tokio::test]
async fn test_update_user() {
    let repo = setup_repository();
    let new_user = NewUser {
        email: "test3@example.com".to_string(),
        name: "Old Name".to_string(),
    };
    let created = repo.create(new_user).await.unwrap();

    let update = UpdateUser {
        name: Some("New Name".to_string()),
        email: None,
    };
    let updated = repo.update(created.id, update).await.unwrap();
    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.email, "test3@example.com");
}

#[tokio::test]
async fn test_delete_user() {
    let repo = setup_repository();
    let new_user = NewUser {
        email: "test4@example.com".to_string(),
        name: "Delete Me".to_string(),
    };
    let created = repo.create(new_user).await.unwrap();

    repo.delete(created.id).await.unwrap();
    let found = repo.find_by_id(created.id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_find_nonexistent_returns_none() {
    let repo = setup_repository();
    let fake_id = Uuid::new_v4();
    let found = repo.find_by_id(fake_id).await.unwrap();
    assert!(found.is_none());
}
