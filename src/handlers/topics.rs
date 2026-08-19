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
    handlers::{approval_decision, assert_body_within_limit, Pagination},
    middleware::ForumUser,
    models::topic::{CreateTopicDto, MergeTopicDto, MoveTopicDto, SplitTopicDto, UpdateTopicDto},
    services::{
        engagement_service::EngagementService, forum_service::ForumService,
        permission_service::PermissionService, post_service::PostService,
        topic_service::TopicService,
    },
    state::AppState,
    events::publisher,
};

pub async fn list_by_forum(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(forum_id): Path<Uuid>,
    Query(page): Query<Pagination>,
) -> Result<Json<Value>> {
    let perms = PermissionService::assert_can_view(forum_id, &user, &state.db).await?;
    let is_mod = perms.is_admin || perms.is_moderator;
    let (limit, offset) = page.resolve(30, 100);
    let topics = TopicService::list_by_forum(forum_id, user.id, is_mod, limit, offset, &state.db).await?;
    let total = TopicService::count_by_forum(forum_id, user.id, is_mod, &state.db).await?;
    Ok(Json(json!({ "topics": topics, "total": total })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(forum_id): Path<Uuid>,
    Json(dto): Json<CreateTopicDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let cfg = state.instance();
    assert_body_within_limit(&dto.body_md, &cfg)?;

    let forum = ForumService::get(forum_id, &state.db).await?;
    let perms = PermissionService::effective(forum_id, &user, &state.db).await?;
    if !perms.can_view || !perms.can_post {
        return Err(ForumError::Forbidden);
    }
    let is_mod = perms.is_admin || perms.is_moderator;
    if (forum.is_locked || forum.is_readonly) && !is_mod {
        return Err(ForumError::Forbidden);
    }
    if !is_mod && crate::services::moderation_service::ModerationService::is_banned(user.id, &state.db).await? {
        return Err(ForumError::Forbidden);
    }
    PostService::assert_not_flooding(user.id, is_mod, &cfg, &state.db).await?;

    // Only moderators/admins may pin (sticky/announcement/global); others get 'normal'.
    let topic_type = match dto.topic_type.as_deref() {
        Some(t) if t != "normal" && !is_mod => return Err(ForumError::Forbidden),
        Some(t) => t.to_string(),
        None => "normal".to_string(),
    };

    let approved = approval_decision(&state, user.id, is_mod, &cfg).await?;
    let (topic, post) = TopicService::create(forum_id, user.id, &topic_type, dto, approved, &state.db).await?;

    // A held topic is not news yet: the bus is told when a moderator releases it.
    if approved {
        publisher::publish_topic_created(&state, topic.id, user.id).await;
        publisher::publish_post_created(&state, post.id, user.id).await;
    }
    Ok((StatusCode::CREATED, Json(json!({ "topic": topic, "post": post, "pending": !approved }))))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let topic = TopicService::get(id, &state.db).await?;
    let perms = PermissionService::assert_can_view(topic.forum_id, &user, &state.db).await?;
    // A topic held for approval is not addressable by its id either: the
    // listing hides it, so the URL must not be the way around that. Its author
    // and the moderators still reach it.
    if !topic.is_approved
        && topic.author_id != user.id
        && !(perms.is_admin || perms.is_moderator)
    {
        return Err(ForumError::NotFound(format!("Topic {id}")));
    }
    TopicService::touch_view(id, &state.db).await?;
    Ok(Json(json!({
        "topic": topic,
        "permissions": {
            "can_reply":    perms.can_reply,
            "can_attach":   perms.can_attach,
            "is_moderator": perms.is_moderator,
            "is_admin":     perms.is_admin,
        }
    })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateTopicDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let topic = TopicService::update(id, &user, dto, &state.db).await?;
    Ok(Json(json!({ "topic": topic })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    TopicService::delete(id, &user, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Moderation actions ────────────────────────────────────────────────────────

async fn assert_topic_moderator(topic_id: Uuid, user: &ForumUser, state: &AppState) -> Result<()> {
    let topic = TopicService::get(topic_id, &state.db).await?;
    let perms = PermissionService::effective(topic.forum_id, user, &state.db).await?;
    if perms.is_admin || perms.is_moderator { Ok(()) } else { Err(ForumError::Forbidden) }
}

pub async fn lock(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_topic_moderator(id, &user, &state).await?;
    let topic = TopicService::set_locked(id, true, &state.db).await?;
    Ok(Json(json!({ "topic": topic })))
}

pub async fn unlock(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_topic_moderator(id, &user, &state).await?;
    let topic = TopicService::set_locked(id, false, &state.db).await?;
    Ok(Json(json!({ "topic": topic })))
}

pub async fn move_topic(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<MoveTopicDto>,
) -> Result<Json<Value>> {
    assert_topic_moderator(id, &user, &state).await?;
    let topic = TopicService::move_to(id, dto, &state.db).await?;
    Ok(Json(json!({ "topic": topic })))
}

pub async fn split(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<SplitTopicDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    assert_topic_moderator(id, &user, &state).await?;
    let topic = TopicService::split(id, user.id, dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "topic": topic }))))
}

pub async fn merge(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<MergeTopicDto>,
) -> Result<Json<Value>> {
    assert_topic_moderator(id, &user, &state).await?;
    let topic = TopicService::merge(id, dto, &state.db).await?;
    Ok(Json(json!({ "topic": topic })))
}

// ── Engagement (subscribe / read) ─────────────────────────────────────────────

pub async fn subscribe(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>)> {
    let sub = EngagementService::subscribe_topic(user.id, id, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "subscription": sub }))))
}

pub async fn unsubscribe(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    EngagementService::unsubscribe_topic(user.id, id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize)]
pub struct MarkReadDto {
    pub last_read_post_id: Option<Uuid>,
}

pub async fn mark_read(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<MarkReadDto>,
) -> Result<StatusCode> {
    EngagementService::mark_read(user.id, id, dto.last_read_post_id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Solution (accepted answer) ─────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct SolutionDto {
    pub post_id: Uuid,
}

pub async fn set_solution(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<SolutionDto>,
) -> Result<Json<Value>> {
    let (topic, post_author) = TopicService::set_solution(id, dto.post_id, &user, &state.db).await?;
    crate::services::notification_service::NotificationService::notify(
        &state, post_author, "solution", user.id, id, Some(dto.post_id), None,
    ).await;
    Ok(Json(json!({ "topic": topic })))
}

pub async fn clear_solution(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let topic = TopicService::clear_solution(id, &user, &state.db).await?;
    Ok(Json(json!({ "topic": topic })))
}
