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
    models::post::{CreatePostDto, UpdatePostDto},
    services::{
        censor_service::CensorService, engagement_service::EngagementService,
        forum_service::ForumService, notification_service::NotificationService,
        permission_service::PermissionService, post_service::PostService,
        topic_service::TopicService,
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
    let perms = PermissionService::assert_can_view(topic.forum_id, &user, &state.db).await?;
    let is_mod = perms.is_admin || perms.is_moderator;
    let (limit, offset) = page.resolve(20, 100);
    let mut posts = PostService::list_by_topic(topic_id, user.id, is_mod, limit, offset, &state.db).await?;
    let total = PostService::count_by_topic(topic_id, user.id, is_mod, &state.db).await?;
    // Word censor (server-side, phpBB-style): substituted here so the response
    // body never carries the raw word — a client-side filter would be trivial
    // to bypass by reading the payload directly.
    for post in &mut posts {
        post.body_md = CensorService::apply(&post.body_md);
    }
    Ok(Json(json!({ "posts": posts, "total": total })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(topic_id): Path<Uuid>,
    Json(dto): Json<CreatePostDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let cfg = state.instance();
    assert_body_within_limit(&dto.body_md, &cfg)?;

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
    PostService::assert_not_flooding(user.id, is_mod, &cfg, &state.db).await?;

    let reply_to = dto.reply_to_post_id;
    let mention_ids = dto.mention_user_ids.clone();
    let approved = approval_decision(&state, user.id, is_mod, &cfg).await?;
    let post = PostService::create(topic_id, topic.forum_id, user.id, dto, approved, &state.db).await?;

    // A message held for approval is not an event and notifies nobody: telling
    // the topic author that someone replied, when nobody can read the reply,
    // announces the queue's contents to a person with no say over it.
    if approved {
        publisher::publish_post_created(&state, post.id, user.id).await;

        // Whether the forum is open to ordinary members. When it is restricted,
        // only its moderators may be notified about it — the same visibility gate
        // mentions use, so no notification ever reveals a topic to someone who
        // cannot reach it (SEC-11).
        let forum_open = PermissionService::role_can_view(topic.forum_id, "user", &state.db)
            .await
            .unwrap_or(false);

        // Track who has been told so a subscriber who is also the topic author or
        // was mentioned is not notified twice.
        let mut notified: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
        notified.insert(user.id);

        // The topic author (always allowed to see their own topic).
        if notified.insert(topic.author_id) {
            NotificationService::notify(&state, topic.author_id, "reply", user.id, topic_id, Some(post.id), None).await;
        }
        // The author of the quoted message.
        if let Some(rid) = reply_to {
            if let Ok(parent) = PostService::get(rid, &state.db).await {
                if notified.insert(parent.author_id) {
                    NotificationService::notify(&state, parent.author_id, "reply", user.id, topic_id, Some(post.id), None).await;
                }
            }
        }
        // Explicit @mentions (visibility-gated).
        for uid in mention_ids {
            if notified.insert(uid) {
                let may_see = forum_open
                    || PermissionService::is_moderator(topic.forum_id, uid, &state.db).await.unwrap_or(false);
                if may_see {
                    NotificationService::notify(&state, uid, "mention", user.id, topic_id, Some(post.id), None).await;
                }
            }
        }
        // Everyone watching this topic or its forum (V2): the subscription rows
        // finally do something. Gated by the same visibility rule, and skipping
        // anyone already notified above.
        if let Ok(watchers) = EngagementService::topic_watchers(topic_id, topic.forum_id, user.id, &state.db).await {
            for uid in watchers {
                if notified.insert(uid) {
                    let may_see = forum_open
                        || PermissionService::is_moderator(topic.forum_id, uid, &state.db).await.unwrap_or(false);
                    if may_see {
                        NotificationService::notify(&state, uid, "reply", user.id, topic_id, Some(post.id), None).await;
                    }
                }
            }
        }
    }
    Ok((StatusCode::CREATED, Json(json!({ "post": post, "pending": !approved }))))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let post = PostService::get(id, &state.db).await?;
    // First, the caller must be able to see the forum at all — a direct link to a
    // post id must not bypass forum visibility.
    let perms = PermissionService::assert_can_view(post.forum_id, &user, &state.db).await?;
    let is_mod = perms.is_admin || perms.is_moderator;
    // Same rule as the listing: a message waiting for approval or removed by
    // moderation is readable only by its author and by moderators — including
    // through a direct link to its id (SEC-02).
    if (!post.is_approved || post.is_deleted) && post.author_id != user.id && !is_mod {
        return Err(ForumError::NotFound(format!("Post {id}")));
    }
    Ok(Json(json!({ "post": post })))
}

/// Edit history of a post: visible only to its author and to moderators, for
/// the same reason a pending/removed post is (SEC-02) — a direct link to a
/// post id must not leak more than the post itself would show.
pub async fn list_revisions(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let post = PostService::get(id, &state.db).await?;
    let perms = PermissionService::assert_can_view(post.forum_id, &user, &state.db).await?;
    let is_mod = perms.is_admin || perms.is_moderator;
    if post.author_id != user.id && !is_mod {
        return Err(ForumError::NotFound(format!("Post {id}")));
    }
    let revisions = PostService::list_revisions(id, &state.db).await?;
    Ok(Json(json!({ "revisions": revisions })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdatePostDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let cfg = state.instance();
    assert_body_within_limit(&dto.body_md, &cfg)?;
    let post = PostService::update(id, &user, dto, &cfg, &state.db).await?;
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
