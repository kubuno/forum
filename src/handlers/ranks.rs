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
    models::rank::{CreateRankDto, UpdateProfileDto, UpdateRankDto, UserProfile},
    services::{
        engagement_service::EngagementService, moderation_service::ModerationService,
        permission_service::PermissionService, rank_service::RankService,
    },
    state::AppState,
};

// ── Ranks ───────────────────────────────────────────────────────────────────

pub async fn list(State(state): State<AppState>) -> Result<Json<Value>> {
    let ranks = RankService::list(&state.db).await?;
    Ok(Json(json!({ "ranks": ranks })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<CreateRankDto>,
) -> Result<(StatusCode, Json<Value>)> {
    PermissionService::assert_admin(&user)?;
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let rank = RankService::create(dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "rank": rank }))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateRankDto>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let rank = RankService::update(id, dto, &state.db).await?;
    Ok(Json(json!({ "rank": rank })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    RankService::delete(id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct AssignRankDto {
    /// The special rank to assign, or `None` to clear it.
    pub rank_id: Option<Uuid>,
}

/// PATCH /profiles/:uid/rank — admin only. Sets a user's manually-assigned
/// SPECIAL rank (`RankService::assign_special` rejects a non-special one).
pub async fn assign_rank(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
    Json(dto): Json<AssignRankDto>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let profile = RankService::assign_special(uid, dto.rank_id, &state.db).await?;
    let details = if dto.rank_id.is_some() { "special rank assigned" } else { "special rank cleared" };
    ModerationService::log(user.id, "rank_assign", None, None, None, Some(uid), Some(details), &state.db).await;
    Ok(Json(json!({ "profile": profile })))
}

// ── Profiles ──────────────────────────────────────────────────────────────────

/// Blanks the signature when the instance has turned signatures off.
///
/// Refusing to SAVE a signature is not enough: an instance that switches them
/// off would keep displaying every signature written before the change under
/// every message their authors ever posted. The stored text is left alone —
/// turning the setting back on restores it — it simply stops being served.
fn apply_signature_policy(profile: &mut UserProfile, allowed: bool) {
    if !allowed {
        profile.signature_md = None;
    }
}

pub async fn get_profile(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<Json<Value>> {
    let mut profile = RankService::get_profile(uid, &state.db).await?;
    apply_signature_policy(&mut profile, state.instance().allow_signatures);
    let topics = crate::services::topic_service::TopicService::by_author(uid, 10, &state.db).await?;
    Ok(Json(json!({ "profile": profile, "topics": topics })))
}

#[derive(Debug, Deserialize)]
pub struct BriefQuery {
    /// Comma-separated user ids.
    pub ids: Option<String>,
}

/// GET /profiles/brief?ids=uid1,uid2 — compact profiles for a post listing's
/// author column, resolved in one request instead of one per author.
pub async fn brief_profiles(
    State(state): State<AppState>,
    Query(q): Query<BriefQuery>,
) -> Result<Json<Value>> {
    let ids: Vec<Uuid> = q
        .ids
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| Uuid::parse_str(s.trim()).ok())
        .take(100)
        .collect();
    if ids.is_empty() {
        return Ok(Json(json!({ "profiles": [] })));
    }
    let profiles = RankService::brief_profiles(&ids, state.instance().allow_signatures, &state.db).await?;
    Ok(Json(json!({ "profiles": profiles })))
}

/// GET /profiles/:uid/activity — a user's recent posts.
pub async fn activity(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<Json<Value>> {
    let posts = RankService::activity(uid, 20, &state.db).await?;
    Ok(Json(json!({ "posts": posts })))
}

pub async fn my_profile(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let mut profile = RankService::get_profile(user.id, &state.db).await?;
    apply_signature_policy(&mut profile, state.instance().allow_signatures);
    Ok(Json(json!({ "profile": profile })))
}

pub async fn update_my_signature(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<UpdateProfileDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;

    // Instance policy, validated before anything reaches the database. An empty
    // string is always accepted: clearing a signature must stay possible even
    // once the administrator has switched the feature off.
    let cfg = state.instance();
    if let Some(sig) = dto.signature_md.as_deref() {
        let written = sig.trim();
        if !written.is_empty() {
            if !cfg.allow_signatures {
                return Err(ForumError::Forbidden);
            }
            if sig.chars().count() > cfg.max_signature_length {
                return Err(ForumError::Validation(format!(
                    "signature trop longue ({} caractères maximum)",
                    cfg.max_signature_length
                )));
            }
        }
    }

    let mut profile = RankService::update_signature(user.id, dto, &state.db).await?;
    apply_signature_policy(&mut profile, cfg.allow_signatures);
    Ok(Json(json!({ "profile": profile })))
}

pub async fn my_subscriptions(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let subscriptions = EngagementService::list_subscriptions(user.id, &state.db).await?;
    Ok(Json(json!({ "subscriptions": subscriptions })))
}
