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
    models::rank::{CreateRankDto, UpdateProfileDto, UpdateRankDto},
    services::{
        engagement_service::EngagementService, permission_service::PermissionService,
        rank_service::RankService,
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

// ── Profiles ──────────────────────────────────────────────────────────────────

pub async fn get_profile(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<Json<Value>> {
    let profile = RankService::get_profile(uid, &state.db).await?;
    let topics = crate::services::topic_service::TopicService::by_author(uid, 10, &state.db).await?;
    Ok(Json(json!({ "profile": profile, "topics": topics })))
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
    let profile = RankService::get_profile(user.id, &state.db).await?;
    Ok(Json(json!({ "profile": profile })))
}

pub async fn update_my_signature(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<UpdateProfileDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let profile = RankService::update_signature(user.id, dto, &state.db).await?;
    Ok(Json(json!({ "profile": profile })))
}

pub async fn my_subscriptions(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let subscriptions = EngagementService::list_subscriptions(user.id, &state.db).await?;
    Ok(Json(json!({ "subscriptions": subscriptions })))
}
