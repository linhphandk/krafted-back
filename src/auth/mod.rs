pub mod controller;
pub mod models;
pub mod ports;
pub mod provider;
pub mod service;

use crate::state::AppState;
use axum::routing::post;

pub fn auth_router() -> axum::Router<AppState> {
    axum::Router::<AppState>::new()
        .route("/auth/register", post(controller::register))
        .route("/auth/login", post(controller::login))
}
