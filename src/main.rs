use http::Method;
use krafted_back::router::create_router;
use krafted_back::shared::config::Config;
use krafted_back::shared::db::{establish_pool, run_migrations};
use krafted_back::shared::image_storage::S3ImageStorage;
use krafted_back::state::AppState;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env().expect("Failed to load config");

    let pool = establish_pool(&config.database_url, config.database_pool_size);
    run_migrations(&pool);

    let region = if config.s3_region.is_empty() {
        None
    } else {
        Some(config.s3_region.clone())
    };
    let image_storage = S3ImageStorage::new(config.s3_endpoint.clone(), region).await;

    let state = AppState::new(
        pool.clone(),
        config.jwt_secret.clone(),
        config.jwt_expiry_minutes,
        image_storage,
        config.s3_bucket,
    );
    let app = create_router(state);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    let app = app.layer(cors);

    let addr = SocketAddr::from((
        config.server_host.parse::<std::net::IpAddr>().unwrap(),
        config.server_port,
    ));
    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
