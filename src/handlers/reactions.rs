use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::reaction::ReactDto,
    services::{
        permission_service::PermissionService, post_service::PostService,
        reaction_service::{ReactionService, ALLOWED_EMOJIS},
        topic_service::TopicService,
    },
    state::AppState,
};

/// POST /posts/:id/react — toggles the requesting user's reaction.
pub async fn react(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(post_id): Path<Uuid>,
    Json(dto): Json<ReactDto>,
) -> Result<Json<Value>> {
    if !ALLOWED_EMOJIS.contains(&dto.emoji.as_str()) {
        return Err(ForumError::Validation("unsupported emoji".into()));
    }
    let post = PostService::get(post_id, &state.db).await?;
    PermissionService::assert_can_view(post.forum_id, &user, &state.db).await?;

    let (added, reactions) = ReactionService::toggle(post_id, user.id, &dto.emoji, &state.db).await?;

    // Notify the post author of a new reaction (not on self-reactions or removals).
    if added && post.author_id != user.id {
        crate::services::notification_service::NotificationService::notify(
            &state, post.author_id, "reaction", user.id, post.topic_id, Some(post_id), Some(&dto.emoji),
        ).await;
    }
    Ok(Json(json!({ "added": added, "reactions": reactions })))
}

/// GET /topics/:id/reactions — per-post aggregates for the whole topic.
pub async fn for_topic(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(topic_id): Path<Uuid>,
) -> Result<Json<Value>> {
    let topic = TopicService::get(topic_id, &state.db).await?;
    PermissionService::assert_can_view(topic.forum_id, &user, &state.db).await?;
    let map = ReactionService::for_topic(topic_id, user.id, &state.db).await?;
    Ok(Json(json!({ "reactions": map })))
}
