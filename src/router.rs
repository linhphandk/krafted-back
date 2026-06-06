use axum::routing::get;
use axum::Json;
use axum::Router;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::api_doc::ApiDoc;
use crate::auth::auth_router;
use crate::listing::listing_router;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api-docs/openapi.json", get(openapi_json))
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .merge(auth_router(&state))
        .merge(listing_router(&state))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
