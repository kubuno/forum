//! Private messages: `/me/pm/*`. See `services/pm_service.rs` for the
//! security model (every read is membership-gated, 404 for non-participants).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::{ForumError, Result},
    handlers::{assert_body_within_limit, Pagination},
    middleware::ForumUser,
    models::pm::{BlockDto, CreateThreadDto, SendMessageDto},
    services::pm_service::PmService,
    state::AppState,
};

/// GET /me/pm — the caller's inbox, most recently active thread first.
pub async fn list_threads(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Query(page): Query<Pagination>,
) -> Result<Json<Value>> {
    let (limit, offset) = page.resolve(30, 100);
    let threads = PmService::list_threads(user.id, limit, offset, &state.db).await?;
    Ok(Json(json!({ "threads": threads })))
}

/// POST /me/pm — starts a new thread with its first message.
pub async fn create_thread(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<CreateThreadDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let cfg = state.instance();
    assert_body_within_limit(&dto.body_md, &cfg)?;

    let (thread, message) =
        PmService::create_thread(&state, user.id, user.is_admin(), dto, &cfg).await?;
    Ok((StatusCode::CREATED, Json(json!({ "thread": thread, "message": message }))))
}

/// GET /me/pm/unread — thread count with at least one unread message.
pub async fn unread_count(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let count = PmService::unread_count(user.id, &state.db).await?;
    Ok(Json(json!({ "count": count })))
}

/// GET /me/pm/blocks — ids the caller has blocked.
pub async fn list_blocks(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let blocked = PmService::list_blocks(user.id, &state.db).await?;
    Ok(Json(json!({ "blocked": blocked })))
}

/// POST /me/pm/blocks — blocks a user (idempotent).
pub async fn create_block(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<BlockDto>,
) -> Result<StatusCode> {
    PmService::block(user.id, dto.user_id, &state.db).await?;
    Ok(StatusCode::CREATED)
}

/// DELETE /me/pm/blocks/:uid — unblocks a user.
pub async fn delete_block(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
) -> Result<StatusCode> {
    PmService::unblock(user.id, uid, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /me/pm/:id — thread header + its messages. 404 for a non-participant.
pub async fn get_thread(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let (thread, messages) = PmService::get_thread(id, user.id, &state.db).await?;
    Ok(Json(json!({ "thread": thread, "messages": messages })))
}

/// POST /me/pm/:id — appends a message to an existing thread.
pub async fn send_message(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<SendMessageDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let cfg = state.instance();
    assert_body_within_limit(&dto.body_md, &cfg)?;

    let message =
        PmService::send_message(&state, id, user.id, user.is_admin(), dto.body_md, &cfg).await?;
    Ok((StatusCode::CREATED, Json(json!({ "message": message }))))
}

/// POST /me/pm/:id/read — marks the thread read up to now for the caller.
pub async fn mark_read(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PmService::mark_read(id, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /me/pm/:id — hides the thread for the caller only.
pub async fn delete_thread(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PmService::delete_for_user(id, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
