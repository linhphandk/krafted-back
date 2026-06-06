pub mod controller;
pub mod models;
pub mod ports;
pub mod repository;
pub mod service;

use crate::shared::middleware::auth_middleware;
use crate::state::AppState;

pub fn favorites_router(state: &AppState) -> axum::Router<AppState> {
    axum::Router::<AppState>::new()
        .route(
            "/api/favorites",
            axum::routing::get(controller::list_favorites),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
}
