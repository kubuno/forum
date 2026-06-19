use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::{ForumError, Result},
    handlers::Pagination,
    middleware::ForumUser,
    models::post::{CreatePostDto, UpdatePostDto},
    services::{
        forum_service::ForumService, notification_service::NotificationService,
        permission_service::PermissionService, post_service::PostService, topic_service::TopicService,
    },
    state::AppState,
    events::publisher,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(topic_id): Path<Uuid>,
    Query(page): Query<Pagination>,
) -> Result<Json<Value>> {
    let topic = TopicService::get(topic_id, &state.db).await?;
    PermissionService::assert_can_view(topic.forum_id, &user, &state.db).await?;
    let (limit, offset) = page.resolve(20, 100);
    let posts = PostService::list_by_topic(topic_id, limit, offset, &state.db).await?;
    let total = PostService::count_by_topic(topic_id, &state.db).await?;
    Ok(Json(json!({ "posts": posts, "total": total })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(topic_id): Path<Uuid>,
    Json(dto): Json<CreatePostDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;

    let topic = TopicService::get(topic_id, &state.db).await?;
    let forum = ForumService::get(topic.forum_id, &state.db).await?;
    let perms = PermissionService::effective(topic.forum_id, &user, &state.db).await?;
    let is_mod = perms.is_admin || perms.is_moderator;
    if !perms.can_view || !perms.can_reply {
        return Err(ForumError::Forbidden);
    }
    if (topic.is_locked || forum.is_locked || forum.is_readonly) && !is_mod {
        return Err(ForumError::Forbidden);
    }
    if !is_mod && crate::services::moderation_service::ModerationService::is_banned(user.id, &state.db).await? {
        return Err(ForumError::Forbidden);
    }

    let reply_to = dto.reply_to_post_id;
    let mention_ids = dto.mention_user_ids.clone();
    let post = PostService::create(topic_id, topic.forum_id, user.id, dto, &state.db).await?;
    publisher::publish_post_created(&state, post.id, user.id).await;

    // Notify the topic author, the replied-to author and any @mentioned users.
    NotificationService::notify(&state, topic.author_id, "reply", user.id, topic_id, Some(post.id), None).await;
    if let Some(rid) = reply_to {
        if let Ok(parent) = PostService::get(rid, &state.db).await {
            NotificationService::notify(&state, parent.author_id, "reply", user.id, topic_id, Some(post.id), None).await;
        }
    }
    for uid in mention_ids {
        NotificationService::notify(&state, uid, "mention", user.id, topic_id, Some(post.id), None).await;
    }
    Ok((StatusCode::CREATED, Json(json!({ "post": post }))))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Value>> {
    let post = PostService::get(id, &state.db).await?;
    Ok(Json(json!({ "post": post })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdatePostDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let post = PostService::update(id, &user, dto, &state.db).await?;
    publisher::publish_post_updated(&state, post.id, user.id).await;
    Ok(Json(json!({ "post": post })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PostService::delete(id, &user, &state.db).await?;
    publisher::publish_post_deleted(&state, id, user.id).await;
    Ok(StatusCode::NO_CONTENT)
}
