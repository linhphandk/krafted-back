pub mod controller;
pub mod models;
pub mod ports;
pub mod repository;
pub mod service;
pub mod state;

pub use state::UserAppState;

use axum::routing::post;

pub fn user_router() -> axum::Router<UserAppState> {
    axum::Router::<UserAppState>::new().route("/users", post(controller::create_user))
}
