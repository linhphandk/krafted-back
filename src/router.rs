use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::api_doc::ApiDoc;
use crate::user::controller::create_user;
use crate::user::UserAppState;

pub fn create_router(state: UserAppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api-docs/openapi.json", get(openapi_json))
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .route("/users", post(create_user))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
