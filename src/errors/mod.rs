use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ForumError {
    #[error("Not authenticated")]
    Unauthorized,

    #[error("Access denied")]
    Forbidden,

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Invalid data: {0}")]
    Validation(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error")]
    Database(#[from] sqlx::Error),

    #[error("Internal error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ForumError {
    fn into_response(self) -> Response {
        let (status, code, msg) = match &self {
            ForumError::Unauthorized  => (StatusCode::UNAUTHORIZED,         "UNAUTHORIZED", self.to_string()),
            ForumError::Forbidden     => (StatusCode::FORBIDDEN,            "FORBIDDEN",    self.to_string()),
            ForumError::NotFound(_)   => (StatusCode::NOT_FOUND,            "NOT_FOUND",    self.to_string()),
            ForumError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION",   self.to_string()),
            ForumError::Conflict(_)   => (StatusCode::CONFLICT,             "CONFLICT",     self.to_string()),
            ForumError::Database(e) => {
                tracing::error!(error = %e, "Database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", "Database error".to_string())
            }
            ForumError::Internal(e) => {
                tracing::error!(error = %e, "Internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Internal error".to_string())
            }
        };
        (status, Json(json!({ "error": code, "message": msg }))).into_response()
    }
}

pub type Result<T> = std::result::Result<T, ForumError>;
