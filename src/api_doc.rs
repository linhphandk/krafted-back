use utoipa::OpenApi;
use utoipa_scalar::Scalar;

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

pub fn scalar_router() -> Scalar<utoipa::openapi::OpenApi> {
    Scalar::new(ApiDoc::openapi())
}
