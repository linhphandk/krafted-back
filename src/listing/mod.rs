pub mod controller;
pub mod models;
pub mod ports;
pub mod repository;
pub mod service;

use crate::state::AppState;

pub fn listing_router() -> axum::Router<AppState> {
    axum::Router::<AppState>::new()
        .route(
            "/api/listings",
            axum::routing::get(controller::list_listings).post(controller::create_listing),
        )
        .route(
            "/api/listings/mine",
            axum::routing::get(controller::seller_listings),
        )
        .route(
            "/api/listings/{id}",
            axum::routing::get(controller::get_listing)
                .patch(controller::update_listing)
                .delete(controller::delete_listing),
        )
        .route(
            "/api/listings/{id}/publish",
            axum::routing::post(controller::publish_listing),
        )
        .route(
            "/api/listings/{id}/pause",
            axum::routing::post(controller::pause_listing),
        )
        .route(
            "/api/categories",
            axum::routing::get(controller::list_categories),
        )
}
