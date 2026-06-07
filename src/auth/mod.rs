pub mod controller;
pub mod email;
pub mod models;
pub mod ports;
pub mod provider;
pub mod repository;
pub mod service;

use crate::shared::middleware::auth_middleware;
use crate::state::AppState;
use axum::routing::{get, post};

pub fn auth_router(state: &AppState) -> axum::Router<AppState> {
    let protected = axum::Router::<AppState>::new()
        .route(
            "/auth/me",
            get(controller::me).patch(controller::update_profile),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    axum::Router::<AppState>::new()
        .route("/auth/register", post(controller::register))
        .route("/auth/login", post(controller::login))
        .route("/auth/logout", post(controller::logout))
        .route("/auth/refresh", post(controller::refresh))
        .merge(protected)
}
