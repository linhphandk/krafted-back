use axum::routing::get;
use axum::Router;

use crate::user::{user_router, UserAppState};

pub fn create_router() -> Router<UserAppState> {
    Router::new()
        .route("/health", get(health_check))
        .merge(user_router())
}

async fn health_check() -> &'static str {
    "OK"
}
