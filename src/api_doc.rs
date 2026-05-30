use utoipa::OpenApi;

use crate::auth::controller::{RegisterRequest, UserResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::auth::controller::register,
    ),
    components(
        schemas(RegisterRequest, UserResponse, crate::shared::errors::ErrorResponse),
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
    ),
)]
pub struct ApiDoc;
