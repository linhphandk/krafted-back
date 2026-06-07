use axum::body::Body;
use axum::http::StatusCode;
use krafted_back::router::create_router;
use krafted_back::shared::db::{establish_pool, run_migrations};
use krafted_back::shared::image_storage::S3ImageStorage;
use krafted_back::state::AppState;
use serde_json::json;
use testcontainers::clients::Cli;
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup(docker: &Cli) -> (testcontainers::Container<'_, Postgres>, axum::Router) {
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let pool = establish_pool(&db_url, 4);
    run_migrations(&pool);

    let image_storage =
        S3ImageStorage::new(Some("http://localhost:19000".to_string()), None, None).await;
    let state = AppState::new(
        pool,
        "test-secret".to_string(),
        15,
        image_storage,
        "test-bucket".to_string(),
        None,
    );
    let app = create_router(state);
    (container, app)
}

async fn register_user(app: &axum::Router, email: &str) -> (Uuid, String) {
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "email": email,
                        "name": "Test User",
                        "password": "password123"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let user_id = json["user"]["id"].as_str().unwrap().to_string();
    let token = json["access_token"].as_str().unwrap().to_string();
    (Uuid::parse_str(&user_id).unwrap(), token)
}

async fn first_category_id(app: &axum::Router) -> String {
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/categories")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let categories: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    categories[0]["id"].as_str().unwrap().to_string()
}

fn create_body(title: &str, category_id: &str) -> String {
    serde_json::to_string(&json!({
        "title": title,
        "description": "A test description",
        "price_cents": 1000,
        "category_id": category_id,
        "condition": "handmade",
        "quantity": 1,
    }))
    .unwrap()
}

async fn create_listing(
    app: &axum::Router,
    title: &str,
    category_id: &str,
    token: &str,
) -> serde_json::Value {
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/listings")
                .header("content-type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(create_body(title, category_id)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn publish_listing(app: &axum::Router, listing_id: &str, token: &str) {
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/listings/{}/publish", listing_id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_add_favorite_unauthenticated() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let listing_id = Uuid::new_v4();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/favorites/{}", listing_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_add_favorite_listing_not_found() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_user_id, token) = register_user(&app, "buyer@example.com").await;
    let listing_id = Uuid::new_v4();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/favorites/{}", listing_id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_add_favorite_listing_not_active() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, seller_token) = register_user(&app, "seller@example.com").await;
    let (_buyer_id, buyer_token) = register_user(&app, "buyer2@example.com").await;
    let category_id = first_category_id(&app).await;
    let listing = create_listing(&app, "Draft Listing", &category_id, &seller_token);
    let listing_id = listing.await["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/favorites/{}", listing_id))
                .header("Authorization", format!("Bearer {}", buyer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_add_favorite_success() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, seller_token) = register_user(&app, "seller3@example.com").await;
    let (_buyer_id, buyer_token) = register_user(&app, "buyer3@example.com").await;
    let category_id = first_category_id(&app).await;
    let listing = create_listing(&app, "Active Listing", &category_id, &seller_token);
    let listing_val = listing.await;
    let listing_id = listing_val["id"].as_str().unwrap().to_string();
    publish_listing(&app, &listing_id, &seller_token).await;

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/favorites/{}", listing_id))
                .header("Authorization", format!("Bearer {}", buyer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let fav: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(fav["listing_id"], listing_id);
}

#[tokio::test]
async fn test_add_favorite_idempotent() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, seller_token) = register_user(&app, "seller4@example.com").await;
    let (_buyer_id, buyer_token) = register_user(&app, "buyer4@example.com").await;
    let category_id = first_category_id(&app).await;
    let listing = create_listing(&app, "Idempotent Listing", &category_id, &seller_token);
    let listing_val = listing.await;
    let listing_id = listing_val["id"].as_str().unwrap().to_string();
    publish_listing(&app, &listing_id, &seller_token).await;

    let resp1 = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/favorites/{}", listing_id))
                .header("Authorization", format!("Bearer {}", buyer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    let resp2 = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/favorites/{}", listing_id))
                .header("Authorization", format!("Bearer {}", buyer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_favorites_empty() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_user_id, token) = register_user(&app, "buyer5@example.com").await;

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/favorites")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 0);
    assert_eq!(result["total"], 0);
}

#[tokio::test]
async fn test_list_favorites_with_items() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, seller_token) = register_user(&app, "seller6@example.com").await;
    let (_buyer_id, buyer_token) = register_user(&app, "buyer6@example.com").await;
    let category_id = first_category_id(&app).await;

    let listing1 = create_listing(&app, "Listing A", &category_id, &seller_token);
    let listing1_val = listing1.await;
    let listing1_id = listing1_val["id"].as_str().unwrap().to_string();
    publish_listing(&app, &listing1_id, &seller_token).await;

    let listing2 = create_listing(&app, "Listing B", &category_id, &seller_token);
    let listing2_val = listing2.await;
    let listing2_id = listing2_val["id"].as_str().unwrap().to_string();
    publish_listing(&app, &listing2_id, &seller_token).await;

    app.clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/favorites/{}", listing1_id))
                .header("Authorization", format!("Bearer {}", buyer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    app.clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/favorites/{}", listing2_id))
                .header("Authorization", format!("Bearer {}", buyer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/favorites")
                .header("Authorization", format!("Bearer {}", buyer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 2);
    assert_eq!(result["total"], 2);
}

#[tokio::test]
async fn test_list_favorites_unauthenticated() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/favorites")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_remove_favorite_unauthenticated() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let listing_id = Uuid::new_v4();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("DELETE")
                .uri(&format!("/api/favorites/{}", listing_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_remove_favorite_not_found() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_user_id, token) = register_user(&app, "buyer7@example.com").await;
    let listing_id = Uuid::new_v4();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("DELETE")
                .uri(&format!("/api/favorites/{}", listing_id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_remove_favorite_success() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, seller_token) = register_user(&app, "seller8@example.com").await;
    let (_buyer_id, buyer_token) = register_user(&app, "buyer8@example.com").await;
    let category_id = first_category_id(&app).await;
    let listing = create_listing(&app, "Removable Listing", &category_id, &seller_token);
    let listing_val = listing.await;
    let listing_id = listing_val["id"].as_str().unwrap().to_string();
    publish_listing(&app, &listing_id, &seller_token).await;

    app.clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/favorites/{}", listing_id))
                .header("Authorization", format!("Bearer {}", buyer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("DELETE")
                .uri(&format!("/api/favorites/{}", listing_id))
                .header("Authorization", format!("Bearer {}", buyer_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}
