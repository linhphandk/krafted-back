use krafted_back::router::create_router;
use krafted_back::shared::config::Config;
use krafted_back::shared::db::{establish_pool, run_migrations};
use krafted_back::user::UserAppState;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env().expect("Failed to load config");

    let pool = establish_pool(&config.database_url, config.database_pool_size);
    run_migrations(&pool);

    let state = UserAppState::new(pool.clone());
    let app = create_router(state);

    let addr = SocketAddr::from((
        config.server_host.parse::<std::net::IpAddr>().unwrap(),
        config.server_port,
    ));
    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
