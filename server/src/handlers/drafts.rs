use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::ForumUser,
    models::draft::SaveDraftDto,
    services::draft_service::DraftService,
    state::AppState,
};

pub async fn save(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<SaveDraftDto>,
) -> Result<Json<Value>> {
    let draft = DraftService::save(user.id, dto, &state.db).await?;
    Ok(Json(json!({ "draft": draft })))
}

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let drafts = DraftService::list(user.id, &state.db).await?;
    Ok(Json(json!({ "drafts": drafts })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    DraftService::delete(user.id, id, &state.db).await?;
    Ok(Json(json!({ "ok": true })))
}
