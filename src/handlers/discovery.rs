use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::ForumUser,
    services::{tag_service::TagService, topic_service::TopicService},
    state::AppState,
};

#[derive(Deserialize)]
pub struct FeedQuery {
    pub kind:   Option<String>,   // recent | unanswered | popular | unread | mine
    pub solved: Option<bool>,
    pub tag:    Option<Uuid>,
    pub limit:  Option<i64>,
    pub offset: Option<i64>,
}

/// GET /feed — cross-forum topic discovery, decorated with tags.
pub async fn feed(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Query(q): Query<FeedQuery>,
) -> Result<Json<Value>> {
    let kind = q.kind.as_deref().unwrap_or("recent");
    let topics = TopicService::feed(
        kind, user.id, q.solved, q.tag, q.limit.unwrap_or(30), q.offset.unwrap_or(0), &state.db,
    ).await?;
    let ids: Vec<Uuid> = topics.iter().map(|t| t.id).collect();
    let tags = TagService::for_topics(&ids, &state.db).await?;
    Ok(Json(json!({ "topics": topics, "tags": tags })))
}
