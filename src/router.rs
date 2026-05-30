use axum::routing::get;
use axum::Router;
use utoipa::OpenApi;
use utoipa_scalar::Scalar;

use crate::api_doc::ApiDoc;
use crate::user::{user_router, UserAppState};

pub fn create_router() -> Router<UserAppState> {
    Router::new()
        .route("/health", get(health_check))
        .merge(user_router())
        .merge(Scalar::new(ApiDoc::openapi()))
}

async fn health_check() -> &'static str {
    "OK"
}
