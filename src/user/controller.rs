use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::shared::errors::AppResult;
use crate::user::UserAppState;

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
}

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
