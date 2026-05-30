use utoipa::OpenApi;

use crate::user::controller::{CreateUserRequest, UserResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::user::controller::create_user,
    ),
    components(
        schemas(CreateUserRequest, UserResponse, crate::shared::errors::ErrorResponse),
    ),
    tags(
        (name = "users", description = "User management endpoints"),
    ),
)]
pub struct ApiDoc;
