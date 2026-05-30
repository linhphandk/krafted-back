use axum::body::Body;
use axum::http::{Request, StatusCode};
use diesel::RunQueryDsl;
use krafted_back::router::create_router;
use krafted_back::shared::db::establish_pool;
use krafted_back::user::UserAppState;
use tower::ServiceExt;

fn setup_app() -> axum::Router {
    let db_url = "postgres://postgres:example@localhost:5432/postgres";
    let pool = establish_pool(db_url, 4);
    let mut conn = pool.get().expect("Failed to get connection");
    diesel::sql_query("DELETE FROM users")
        .execute(&mut conn)
        .ok();
    let state = UserAppState::new(pool.clone());
    create_router().with_state(state)
}

#[tokio::test]
async fn test_create_user_success() {
    let app = setup_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"test1@example.com","name":"Test User"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_user_empty_email() {
    let app = setup_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"","name":"Test User"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_user_duplicate_email() {
    let app = setup_app();
    let email = format!("dup-{}@example.com", uuid::Uuid::new_v4());

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{}","name":"First"}}"#,
                    email
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{}","name":"Second"}}"#,
                    email
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
