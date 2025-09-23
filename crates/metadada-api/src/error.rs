use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use utoipa::ToSchema;

pub type AppResult<T> = Result<T, AppError>;

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Internal(err.into().to_string())
    }
}

#[derive(Debug, ToSchema)]
pub enum AppError {
    #[schema(example = "Internal error")]
    Internal(String),
    #[schema(example = "Ressource not found")]
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Internal(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": err.to_string()
                })),
            ),
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "Resource  not found"
                })),
            ),
        }
        .into_response()
    }
}
