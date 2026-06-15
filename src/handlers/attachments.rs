use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::attachment::CreateAttachmentDto,
    services::attachment_service::AttachmentService,
    state::AppState,
};

pub async fn list(State(state): State<AppState>, Path(post_id): Path<Uuid>) -> Result<Json<Value>> {
    let attachments = AttachmentService::list(post_id, &state.db).await?;
    Ok(Json(json!({ "attachments": attachments })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(post_id): Path<Uuid>,
    Json(dto): Json<CreateAttachmentDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let attachment = AttachmentService::create(post_id, &user, dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "attachment": attachment }))))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    AttachmentService::delete(id, &user, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
