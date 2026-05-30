pub mod controller;
pub mod models;
pub mod ports;
pub mod repository;
pub mod service;
pub mod state;

pub use state::UserAppState;

use axum::routing::post;
use axum::Router;

pub fn user_router() -> Router<UserAppState> {
    Router::new().route("/users", post(controller::create_user))
}
