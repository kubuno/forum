use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::ForumUser,
    services::{bookmark_service::BookmarkService, permission_service::PermissionService, topic_service::TopicService},
    state::AppState,
};

/// POST /topics/:id/bookmark — toggles a bookmark.
pub async fn toggle(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(topic_id): Path<Uuid>,
) -> Result<Json<Value>> {
    let topic = TopicService::get(topic_id, &state.db).await?;
    PermissionService::assert_can_view(topic.forum_id, &user, &state.db).await?;
    let bookmarked = BookmarkService::toggle(user.id, topic_id, &state.db).await?;
    Ok(Json(json!({ "bookmarked": bookmarked })))
}

/// GET /me/bookmarks — the user's bookmarked topics.
pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let topics = BookmarkService::list(user.id, &state.db).await?;
    Ok(Json(json!({ "topics": topics })))
}
