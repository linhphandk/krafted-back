use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::shared::errors::AppResult;
use crate::user::UserAppState;

#[derive(Deserialize, ToSchema)]
pub struct CreateUserRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "John Doe")]
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
}

#[utoipa::path(
    post,
    path = "/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = UserResponse),
        (status = 400, description = "Bad request", body = crate::shared::errors::ErrorResponse),
    ),
    tag = "users",
)]
pub async fn create_user(
    State(state): State<UserAppState>,
    Json(req): Json<CreateUserRequest>,
) -> AppResult<(StatusCode, Json<UserResponse>)> {
    let user = state.service.create_user(req.email, req.name).await?;
    let response = UserResponse {
        id: user.id.to_string(),
        email: user.email,
        name: user.name,
    };
    Ok((StatusCode::CREATED, Json(response)))
}
