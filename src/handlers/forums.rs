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
    models::forum::{CreateForumDto, UpdateForumDto},
    services::{
        engagement_service::EngagementService, forum_service::ForumService,
        permission_service::PermissionService,
    },
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListForumsQuery {
    pub category_id: Option<Uuid>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListForumsQuery>,
) -> Result<Json<Value>> {
    let forums = match q.category_id {
        Some(cid) => ForumService::list_by_category(cid, &state.db).await?,
        None => ForumService::list(&state.db).await?,
    };
    Ok(Json(json!({ "forums": forums })))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let forum = ForumService::get(id, &state.db).await?;
    let perms = PermissionService::assert_can_view(id, &user, &state.db).await?;
    Ok(Json(json!({
        "forum": forum,
        "permissions": {
            "can_post":     perms.can_post,
            "can_reply":    perms.can_reply,
            "can_attach":   perms.can_attach,
            "is_moderator": perms.is_moderator,
            "is_admin":     perms.is_admin,
        }
    })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<CreateForumDto>,
) -> Result<(StatusCode, Json<Value>)> {
    PermissionService::assert_admin(&user)?;
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let forum = ForumService::create(dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "forum": forum }))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateForumDto>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let forum = ForumService::update(id, dto, &state.db).await?;
    Ok(Json(json!({ "forum": forum })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    ForumService::delete(id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn read_state(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let markers = EngagementService::read_state_for_forum(user.id, id, &state.db).await?;
    Ok(Json(json!({ "read_state": markers })))
}

pub async fn subscribe(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>)> {
    let sub = EngagementService::subscribe_forum(user.id, id, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "subscription": sub }))))
}

pub async fn unsubscribe(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    EngagementService::unsubscribe_forum(user.id, id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
