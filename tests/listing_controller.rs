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

    let image_storage = S3ImageStorage::new(Some("http://localhost:19000".to_string())).await;
    let state = AppState::new(
        pool,
        "test-secret".to_string(),
        15,
        image_storage,
        "test-bucket".to_string(),
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
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "create_listing({}) failed with {}",
        title,
        resp.status()
    );
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn publish_listing(app: &axum::Router, listing_id: &str, token: &str) {
    app.clone()
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
}

// ─── Categories ────────────────────────────────────────────────

#[tokio::test]
async fn test_list_categories() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
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

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let categories: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!categories.is_empty());
    assert_eq!(categories[0]["kind"], "craft");
}

#[tokio::test]
async fn test_list_categories_filter_by_kind() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/categories?kind=supply")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let categories: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!categories.is_empty());
    for cat in &categories {
        assert_eq!(cat["kind"], "supply");
    }
}

// ─── Listings — public ─────────────────────────────────────────

#[tokio::test]
async fn test_list_listings_empty() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/listings")
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
async fn test_create_listing_requires_auth() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let cid = first_category_id(&app).await;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/listings")
                .header("content-type", "application/json")
                .body(Body::from(create_body("No Auth", &cid)))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_listing_invalid_body() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "invalid-body@test.com").await;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/listings")
                .header("content-type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    serde_json::to_string(&json!({"title": "only title"})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_create_and_get_listing() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (seller_id, token) = register_user(&app, "create-get@test.com").await;
    let cid = first_category_id(&app).await;

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/listings")
                .header("content-type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(create_body("Handmade Vase", &cid)))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(created["title"], "Handmade Vase");
    assert_eq!(created["price_cents"], 1000);
    assert_eq!(created["status"], "draft");
    assert_eq!(created["seller_id"], seller_id.to_string());
    assert_eq!(created["seller_name"], "Test User");
    let listing_id = created["id"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri(&format!("/api/listings/{}", listing_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let got: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(got["title"], "Handmade Vase");
    assert!(got["category_name"].as_str().unwrap().len() > 0);
    assert_eq!(got["seller_name"], "Test User");
}

#[tokio::test]
async fn test_get_listing_not_found() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/listings/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ─── Ownership checks ──────────────────────────────────────────

#[tokio::test]
async fn test_update_listing_owner_only() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "owner1@test.com").await;
    let (_other_id, other_token) = register_user(&app, "owner2@test.com").await;
    let cid = first_category_id(&app).await;
    let created = create_listing(&app, "My Item", &cid, &token).await;
    let listing_id = created["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("PATCH")
                .uri(&format!("/api/listings/{}", listing_id))
                .header("content-type", "application/json")
                .header("Authorization", format!("Bearer {}", other_token))
                .body(Body::from(
                    serde_json::to_string(&json!({"title": "Hacked"})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_delete_listing_owner_only() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "del-owner@test.com").await;
    let (_other_id, other_token) = register_user(&app, "del-other@test.com").await;
    let cid = first_category_id(&app).await;
    let created = create_listing(&app, "To Delete", &cid, &token).await;
    let listing_id = created["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("DELETE")
                .uri(&format!("/api/listings/{}", listing_id))
                .header("Authorization", format!("Bearer {}", other_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_publish_listing_owner_only() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "pub-owner@test.com").await;
    let (_other_id, other_token) = register_user(&app, "pub-other@test.com").await;
    let cid = first_category_id(&app).await;
    let created = create_listing(&app, "Secret", &cid, &token).await;
    let listing_id = created["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/listings/{}/publish", listing_id))
                .header("Authorization", format!("Bearer {}", other_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ─── Lifecycle ─────────────────────────────────────────────────

#[tokio::test]
async fn test_publish_and_list_active() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "publish@test.com").await;
    let cid = first_category_id(&app).await;
    let created = create_listing(&app, "Publishable Item", &cid, &token).await;
    let listing_id = created["id"].as_str().unwrap().to_string();

    publish_listing(&app, &listing_id, &token).await;

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/listings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    assert_eq!(result["items"][0]["title"], "Publishable Item");
}

#[tokio::test]
async fn test_pause_listing() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "pause@test.com").await;
    let cid = first_category_id(&app).await;
    let created = create_listing(&app, "Pausable", &cid, &token).await;
    let listing_id = created["id"].as_str().unwrap().to_string();
    publish_listing(&app, &listing_id, &token).await;

    app.clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(&format!("/api/listings/{}/pause", listing_id))
                .header("Authorization", format!("Bearer {}", token))
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
                .uri("/api/listings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_delete_listing() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "delete@test.com").await;
    let cid = first_category_id(&app).await;
    let created = create_listing(&app, "To Delete", &cid, &token).await;
    let listing_id = created["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("DELETE")
                .uri(&format!("/api/listings/{}", listing_id))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri(&format!("/api/listings/{}", listing_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ─── Seller listings ───────────────────────────────────────────

#[tokio::test]
async fn test_seller_listings() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "seller@test.com").await;
    let cid = first_category_id(&app).await;

    for i in 0..3 {
        create_listing(&app, &format!("Item {}", i), &cid, &token).await;
    }

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/listings/mine")
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
    assert_eq!(result["items"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_seller_listings_requires_auth() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/listings/mine")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ─── Pagination ────────────────────────────────────────────────

#[tokio::test]
async fn test_listings_pagination() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "pagination@test.com").await;
    let cid = first_category_id(&app).await;

    for i in 0..5 {
        let created = create_listing(&app, &format!("Paginated {}", i), &cid, &token).await;
        let lid = created["id"].as_str().unwrap().to_string();
        publish_listing(&app, &lid, &token).await;
    }

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/listings?page=1&per_page=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 2);
    assert_eq!(result["total"], 5);
    assert_eq!(result["page"], 1);
    assert_eq!(result["per_page"], 2);
    assert!(result["total_pages"].as_i64().unwrap() >= 3);

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/listings?page=3&per_page=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
}

// ─── Search & sort ─────────────────────────────────────────────

#[tokio::test]
async fn test_search_listings() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "search@test.com").await;
    let cid = first_category_id(&app).await;

    for title in &["Red Pottery", "Blue Pottery", "Wooden Bowl"] {
        let created = create_listing(&app, title, &cid, &token).await;
        let lid = created["id"].as_str().unwrap().to_string();
        publish_listing(&app, &lid, &token).await;
    }

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/listings?search=pottery")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_sort_by_price() {
    let docker = Cli::default();
    let (_container, app) = setup(&docker).await;
    let (_seller_id, token) = register_user(&app, "sort@test.com").await;
    let cid = first_category_id(&app).await;

    let prices = [500, 1500, 3000];

    for (i, price) in prices.iter().enumerate() {
        let body = serde_json::to_string(&json!({
            "title": format!("Item {}", i),
            "description": "desc",
            "price_cents": price,
            "category_id": cid,
            "condition": "handmade",
            "quantity": 1,
        }))
        .unwrap();
        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/listings")
                    .header("content-type", "application/json")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let lid = created["id"].as_str().unwrap().to_string();
        publish_listing(&app, &lid, &token).await;
    }

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/listings?sort=price_asc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = result["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["price_cents"], 500);
    assert_eq!(items[2]["price_cents"], 3000);

    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/listings?sort=price_desc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = result["items"].as_array().unwrap();
    assert_eq!(items[0]["price_cents"], 3000);
    assert_eq!(items[2]["price_cents"], 500);
}
