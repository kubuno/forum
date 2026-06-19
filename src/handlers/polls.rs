use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
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
    let results = PollService::vote(poll_id, user.id, &dto.option_ids, &state.db).await?;
    Ok(Json(json!({ "poll": results })))
}
