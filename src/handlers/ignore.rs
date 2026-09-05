//! Ignore list: `/me/ignored`. Purely a display preference — see
//! `services/ignore_service.rs` for the security model.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result, middleware::ForumUser, models::ignore::IgnoreDto,
    services::ignore_service::IgnoreService, state::AppState,
};

/// GET /me/ignored — the caller's own ignore list.
pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let ignored = IgnoreService::list(user.id, &state.db).await?;
    Ok(Json(json!({ "ignored": ignored })))
}

/// POST /me/ignored — ignores a member (idempotent).
pub async fn add(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<IgnoreDto>,
) -> Result<StatusCode> {
    IgnoreService::add(user.id, dto.user_id, &state.db).await?;
    Ok(StatusCode::CREATED)
}

/// DELETE /me/ignored/:uid — stops ignoring a member.
pub async fn remove(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
) -> Result<StatusCode> {
    IgnoreService::remove(user.id, uid, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
