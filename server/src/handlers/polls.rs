use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::poll::VoteDto,
    services::{permission_service::PermissionService, poll_service::PollService, topic_service::TopicService},
    state::AppState,
};

/// GET /topics/:id/poll — the topic's poll with results, or null.
pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(topic_id): Path<Uuid>,
) -> Result<Json<Value>> {
    let topic = TopicService::get(topic_id, &state.db).await?;
    PermissionService::assert_can_view(topic.forum_id, &user, &state.db).await?;
    let results = PollService::results(topic_id, user.id, &state.db).await?;
    Ok(Json(json!({ "poll": results })))
}

/// POST /polls/:id/vote
pub async fn vote(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(poll_id): Path<Uuid>,
    Json(dto): Json<VoteDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    crate::services::moderation_service::ModerationService::assert_not_banned(user.id, &state.db).await?;

    // A vote is a write against the poll's topic: it must obey the same access
    // rules as posting there — the caller can see the forum, the topic is not
    // locked, and it is not still waiting for approval (SEC-03).
    let poll = PollService::find(poll_id, &state.db).await?;
    let topic = TopicService::get(poll.topic_id, &state.db).await?;
    let perms = PermissionService::assert_can_view(topic.forum_id, &user, &state.db).await?;
    let is_mod = perms.is_admin || perms.is_moderator;
    if topic.is_locked && !is_mod {
        return Err(ForumError::Forbidden);
    }
    if !topic.is_approved && topic.author_id != user.id && !is_mod {
        return Err(ForumError::NotFound(format!("Poll {poll_id}")));
    }

    let results = PollService::vote(poll_id, user.id, &dto.option_ids, &state.db).await?;
    Ok(Json(json!({ "poll": results })))
}
