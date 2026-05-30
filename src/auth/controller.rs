use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::shared::errors::AppResult;
use crate::state::AppState;

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "John Doe")]
    pub name: String,
    #[schema(example = "securepassword123")]
    pub password: String,
}

#[derive(serde::Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
}

#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = UserResponse),
        (status = 400, description = "Bad request", body = crate::shared::errors::ErrorResponse),
    ),
    tag = "auth",
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<UserResponse>)> {
    let user = state
        .auth_service
        .register(req.email, req.name, req.password)
        .await?;
    let response = UserResponse {
        id: user.id.to_string(),
        email: user.email,
        name: user.name,
    };
    Ok((StatusCode::CREATED, Json(response)))
}
