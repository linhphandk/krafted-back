pub mod controller;
pub mod models;
pub mod ports;
pub mod repository;
pub mod service;
pub mod state;

pub use state::UserAppState;

use crate::state::AppState;
use axum::routing::post;

pub fn user_router() -> axum::Router<AppState> {
    axum::Router::<AppState>::new().route("/users", post(controller::create_user))
}
