use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::moderation::{AddModeratorDto, BanDto, CreateReportDto, ModNoteDto, ResolveReportDto, WarnDto},
    services::{moderation_service::ModerationService, permission_service::PermissionService, post_service::PostService},
    state::AppState,
    events::publisher,
};

// ── Reports ───────────────────────────────────────────────────────────────────

pub async fn report_post(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(post_id): Path<Uuid>,
    Json(dto): Json<CreateReportDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let report = ModerationService::report_post(post_id, user.id, dto, &state.db).await?;
    publisher::publish_reported(&state, post_id, user.id).await;
    Ok((StatusCode::CREATED, Json(json!({ "report": report }))))
}

/// True when the user is an admin or moderates at least one forum.
async fn assert_can_moderate(user: &ForumUser, state: &AppState) -> Result<()> {
    if user.is_admin() {
        return Ok(());
    }
    let any: Option<i32> = sqlx::query_scalar("SELECT 1 FROM forum.moderators WHERE user_id = $1 LIMIT 1")
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?;
    if any.is_some() { Ok(()) } else { Err(ForumError::Forbidden) }
}

#[derive(Debug, Deserialize)]
pub struct ReportsQuery {
    pub status: Option<String>,
}

pub async fn list_reports(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Query(q): Query<ReportsQuery>,
) -> Result<Json<Value>> {
    assert_can_moderate(&user, &state).await?;
    let reports = ModerationService::list_reports(q.status, &state.db).await?;
    Ok(Json(json!({ "reports": reports })))
}

pub async fn resolve_report(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ResolveReportDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    // The handler must moderate the forum the reported post belongs to.
    let forum_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT p.forum_id FROM forum.reports r JOIN forum.posts p ON p.id = r.post_id WHERE r.id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let forum_id = forum_id.ok_or_else(|| ForumError::NotFound(format!("Report {id}")))?;
    let perms = PermissionService::effective(forum_id, &user, &state.db).await?;
    if !perms.is_admin && !perms.is_moderator {
        return Err(ForumError::Forbidden);
    }
    let report = ModerationService::resolve_report(id, user.id, dto, &state.db).await?;
    Ok(Json(json!({ "report": report })))
}

// ── Moderators (per forum) ────────────────────────────────────────────────────

pub async fn list_moderators(
    State(state): State<AppState>,
    Path(forum_id): Path<Uuid>,
) -> Result<Json<Value>> {
    let moderators = ModerationService::list_moderators(forum_id, &state.db).await?;
    Ok(Json(json!({ "moderators": moderators })))
}

pub async fn add_moderator(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(forum_id): Path<Uuid>,
    Json(dto): Json<AddModeratorDto>,
) -> Result<(StatusCode, Json<Value>)> {
    PermissionService::assert_admin(&user)?;
    let moderator = ModerationService::add_moderator(forum_id, dto.user_id, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "moderator": moderator }))))
}

pub async fn remove_moderator(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path((forum_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    ModerationService::remove_moderator(forum_id, user_id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Moderation log ──────────────────────────────────────────────────────────

pub async fn mod_log(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    assert_can_moderate(&user, &state).await?;
    let entries = ModerationService::list_log(100, &state.db).await?;
    Ok(Json(json!({ "log": entries })))
}

// ── Warnings ────────────────────────────────────────────────────────────────

pub async fn warn_user(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
    Json(dto): Json<WarnDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    assert_can_moderate(&user, &state).await?;
    let warning = ModerationService::warn(uid, user.id, &dto.reason, &state.db).await?;
    Ok(Json(json!({ "warning": warning })))
}

pub async fn list_warnings(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_can_moderate(&user, &state).await?;
    let warnings = ModerationService::list_warnings(uid, &state.db).await?;
    Ok(Json(json!({ "warnings": warnings })))
}

// ── Bans (admin only) ───────────────────────────────────────────────────────

pub async fn ban_user(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
    Json(dto): Json<BanDto>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let ban = ModerationService::ban(uid, user.id, dto.reason.as_deref(), dto.days, &state.db).await?;
    Ok(Json(json!({ "ban": ban })))
}

pub async fn unban_user(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    ModerationService::unban(uid, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_bans(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    assert_can_moderate(&user, &state).await?;
    let bans = ModerationService::list_bans(&state.db).await?;
    Ok(Json(json!({ "bans": bans })))
}

// ── Private notes ───────────────────────────────────────────────────────────

pub async fn add_note(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<ModNoteDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    assert_can_moderate(&user, &state).await?;
    let note = ModerationService::add_note(user.id, dto.target_user_id, dto.topic_id, dto.post_id, &dto.body, &state.db).await?;
    Ok(Json(json!({ "note": note })))
}

pub async fn list_notes(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_can_moderate(&user, &state).await?;
    let notes = ModerationService::list_notes(uid, &state.db).await?;
    Ok(Json(json!({ "notes": notes })))
}

// ── Soft delete / restore ───────────────────────────────────────────────────

async fn assert_can_moderate_post(post_id: Uuid, user: &ForumUser, state: &AppState) -> Result<()> {
    let post = PostService::get(post_id, &state.db).await?;
    let perms = PermissionService::effective(post.forum_id, user, &state.db).await?;
    if perms.is_admin || perms.is_moderator { Ok(()) } else { Err(ForumError::Forbidden) }
}

pub async fn remove_post(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    assert_can_moderate_post(id, &user, &state).await?;
    ModerationService::soft_delete_post(id, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_post(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_can_moderate_post(id, &user, &state).await?;
    ModerationService::restore_post(id, user.id, &state.db).await?;
    Ok(Json(json!({ "ok": true })))
}
