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
    models::tag::{CreateTagDto, SetTopicTagsDto},
    services::{permission_service::PermissionService, tag_service::TagService, topic_service::TopicService},
    state::AppState,
};

pub async fn list(State(state): State<AppState>) -> Result<Json<Value>> {
    let tags = TagService::list(&state.db).await?;
    Ok(Json(json!({ "tags": tags })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<CreateTagDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    PermissionService::assert_admin(&user)?;
    let tag = TagService::create(dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "tag": tag }))))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    TagService::delete(id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /topics/:id/tags
pub async fn for_topic(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Value>> {
    let tags = TagService::for_topic(id, &state.db).await?;
    Ok(Json(json!({ "tags": tags })))
}

/// PUT /topics/:id/tags — author or moderator replaces a topic's tags.
pub async fn set_for_topic(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<SetTopicTagsDto>,
) -> Result<Json<Value>> {
    let topic = TopicService::get(id, &state.db).await?;
    let perms = PermissionService::effective(topic.forum_id, &user, &state.db).await?;
    if topic.author_id != user.id && !perms.is_admin && !perms.is_moderator {
        return Err(ForumError::Forbidden);
    }
    let tags = TagService::set_topic_tags(id, &dto.tag_ids, &state.db).await?;
    Ok(Json(json!({ "tags": tags })))
}
