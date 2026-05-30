use axum::routing::get;
use axum::Json;
use axum::Router;

use crate::api_doc::OPENAPI_SPEC;
use crate::user::{user_router, UserAppState};

pub fn create_router() -> Router<UserAppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/api-docs/openapi.json", get(openapi_json))
        .merge(user_router())
}

async fn health_check() -> &'static str {
    "OK"
}

async fn openapi_json() -> Json<&'static utoipa::openapi::OpenApi> {
    Json(&OPENAPI_SPEC)
}
