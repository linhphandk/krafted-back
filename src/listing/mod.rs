pub mod controller;
pub mod models;
pub mod ports;
pub mod repository;
pub mod service;

use crate::favorites::controller as favorites_controller;
use crate::shared::middleware::auth_middleware;
use crate::state::AppState;

pub fn listing_router(state: &AppState) -> axum::Router<AppState> {
    let public_routes = axum::Router::<AppState>::new()
        .route(
            "/api/listings",
            axum::routing::get(controller::list_listings),
        )
        .route(
            "/api/listings/{id}",
            axum::routing::get(controller::get_listing),
        )
        .route(
            "/api/listings/{id}/images",
            axum::routing::get(controller::list_images),
        )
        .route(
            "/api/categories",
            axum::routing::get(controller::list_categories),
        );

    let protected_routes = axum::Router::<AppState>::new()
        .route(
            "/api/listings",
            axum::routing::post(controller::create_listing),
        )
        .route(
            "/api/listings/mine",
            axum::routing::get(controller::seller_listings),
        )
        .route(
            "/api/listings/{id}",
            axum::routing::patch(controller::update_listing).delete(controller::delete_listing),
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
            "/api/listings/{id}/images",
            axum::routing::post(controller::upload_images),
        )
        .route(
            "/api/listings/{id}/images/reorder",
            axum::routing::put(controller::reorder_images),
        )
        .route(
            "/api/listings/{id}/images/{image_id}",
            axum::routing::delete(controller::delete_image),
        )
        .route(
            "/api/listings/{id}/favorite",
            axum::routing::post(favorites_controller::add_favorite)
                .delete(favorites_controller::remove_favorite),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    public_routes.merge(protected_routes)
}
