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
    models::faq::{CreateFaqDto, UpdateFaqDto},
    services::{faq_service::FaqService, moderation_service::ModerationService, permission_service::PermissionService},
    state::AppState,
};

/// GET /faq — every authenticated member reads the FAQ; there is nothing
/// sensitive in it, only an admin may change it.
pub async fn list(State(state): State<AppState>) -> Result<Json<Value>> {
    let entries = FaqService::list(&state.db).await?;
    Ok(Json(json!({ "entries": entries })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<CreateFaqDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    PermissionService::assert_admin(&user)?;
    let entry = FaqService::create(dto, &state.db).await?;
    ModerationService::log(user.id, "faq_entry_added", None, None, None, None, Some(&entry.question), &state.db).await;
    Ok((StatusCode::CREATED, Json(json!({ "entry": entry }))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateFaqDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    PermissionService::assert_admin(&user)?;
    let entry = FaqService::update(id, dto, &state.db).await?;
    ModerationService::log(user.id, "faq_entry_updated", None, None, None, None, Some(&entry.question), &state.db).await;
    Ok(Json(json!({ "entry": entry })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    FaqService::delete(id, &state.db).await?;
    ModerationService::log(user.id, "faq_entry_removed", None, None, None, None, None, &state.db).await;
    Ok(StatusCode::NO_CONTENT)
}
