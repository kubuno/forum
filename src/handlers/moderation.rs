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
    models::moderation::{
        AddModeratorDto, BanDto, CreateCensoredWordDto, CreateReportDto, CreateReportReasonDto,
        EmailBanDto, IpBanDto, ModNoteDto, ResolveReportDto, WarnDto,
    },
    services::{
        censor_service::CensorService, moderation_service::ModerationService,
        notification_service::NotificationService, permission_service::PermissionService,
        post_service::PostService, topic_service::TopicService,
    },
    state::AppState,
    events::publisher,
};

// ── Reports ───────────────────────────────────────────────────────────────────

pub async fn report_post(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(post_id): Path<Uuid>,
    Json(dto): Json<CreateReportDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    ModerationService::assert_not_banned(user.id, &state.db).await?;
    let report = ModerationService::report_post(post_id, user.id, dto, &state.db).await?;
    publisher::publish_reported(&state, post_id, user.id).await;
    // Tell the forum's moderators there is something in the queue.
    if let Ok(post) = PostService::get(post_id, &state.db).await {
        if let Ok(mods) = ModerationService::list_moderators(post.forum_id, &state.db).await {
            for m in mods {
                NotificationService::notify(&state, m.user_id, "report", user.id, post.topic_id, Some(post_id), None).await;
            }
        }
    }
    Ok((StatusCode::CREATED, Json(json!({ "report": report }))))
}

// ── Predefined report reasons (admin-curated chip list) ─────────────────────────

/// GET /report-reasons — every authenticated member sees the chip list, so it
/// can be offered at the moment they file a report.
pub async fn list_report_reasons(State(state): State<AppState>) -> Result<Json<Value>> {
    let reasons = ModerationService::list_report_reasons(&state.db).await?;
    Ok(Json(json!({ "reasons": reasons })))
}

pub async fn create_report_reason(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<CreateReportReasonDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    PermissionService::assert_admin(&user)?;
    let reason = ModerationService::create_report_reason(dto, &state.db).await?;
    ModerationService::log(user.id, "report_reason_added", None, None, None, None, Some(&reason.title), &state.db).await;
    Ok((StatusCode::CREATED, Json(json!({ "reason": reason }))))
}

pub async fn delete_report_reason(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    ModerationService::delete_report_reason(id, &state.db).await?;
    ModerationService::log(user.id, "report_reason_removed", None, None, None, None, None, &state.db).await;
    Ok(StatusCode::NO_CONTENT)
}

// ── Word censor (admin-curated, applied server-side at render time) ─────────

/// GET /censored-words — any authenticated member can see the active list
/// (there is nothing sensitive in it — it is applied to everyone's view of
/// every post regardless), only an admin can change it.
pub async fn list_censored_words(State(state): State<AppState>) -> Result<Json<Value>> {
    let words = CensorService::list(&state.db).await?;
    Ok(Json(json!({ "words": words })))
}

pub async fn create_censored_word(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<CreateCensoredWordDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    PermissionService::assert_admin(&user)?;
    let word = CensorService::create(dto, &state.db).await?;
    ModerationService::log(user.id, "censored_word_added", None, None, None, None, Some(&word.pattern), &state.db).await;
    Ok((StatusCode::CREATED, Json(json!({ "word": word }))))
}

pub async fn delete_censored_word(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    CensorService::delete(id, &state.db).await?;
    ModerationService::log(user.id, "censored_word_removed", None, None, None, None, None, &state.db).await;
    Ok(StatusCode::NO_CONTENT)
}

/// True when the user is an admin or moderates at least one forum.
async fn assert_can_moderate(user: &ForumUser, state: &AppState) -> Result<()> {
    if user.is_admin() {
        return Ok(());
    }
    let any: Option<i32> = sqlx::query_scalar("SELECT 1 FROM forum.moderators WHERE user_id = $1 LIMIT 1")
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?;
    if any.is_some() { Ok(()) } else { Err(ForumError::Forbidden) }
}

#[derive(Debug, Deserialize)]
pub struct ReportsQuery {
    pub status: Option<String>,
}

pub async fn list_reports(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Query(q): Query<ReportsQuery>,
) -> Result<Json<Value>> {
    // `moderation_scope` already refuses a caller who moderates nothing, and
    // returns exactly the forums whose reports they may see (None = admin).
    let scope = moderation_scope(&user, &state).await?;
    let reports = ModerationService::list_reports(q.status, scope.as_deref(), &state.db).await?;
    Ok(Json(json!({ "reports": reports })))
}

pub async fn resolve_report(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ResolveReportDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    // The handler must moderate the forum the reported post belongs to.
    let forum_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT p.forum_id FROM forum.reports r JOIN forum.posts p ON p.id = r.post_id WHERE r.id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let forum_id = forum_id.ok_or_else(|| ForumError::NotFound(format!("Report {id}")))?;
    let perms = PermissionService::effective(forum_id, &user, &state.db).await?;
    if !perms.is_admin && !perms.is_moderator {
        return Err(ForumError::Forbidden);
    }
    let report = ModerationService::resolve_report(id, user.id, dto, &state.db).await?;
    ModerationService::log(
        user.id, "resolve_report", Some(forum_id), None, Some(report.post_id),
        None, Some(report.status.as_str()), &state.db,
    ).await;
    // Tell the reporter what became of their report.
    if let Ok(post) = PostService::get(report.post_id, &state.db).await {
        NotificationService::notify(&state, report.reporter_id, "report_resolved", user.id, post.topic_id, Some(report.post_id), None).await;
    }
    Ok(Json(json!({ "report": report })))
}

// ── Moderators (per forum) ────────────────────────────────────────────────────

pub async fn list_moderators(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(forum_id): Path<Uuid>,
) -> Result<Json<Value>> {
    // The roster of a forum the caller cannot even see is not theirs to read.
    PermissionService::assert_can_view(forum_id, &user, &state.db).await?;
    let moderators = ModerationService::list_moderators(forum_id, &state.db).await?;
    Ok(Json(json!({ "moderators": moderators })))
}

pub async fn add_moderator(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(forum_id): Path<Uuid>,
    Json(dto): Json<AddModeratorDto>,
) -> Result<(StatusCode, Json<Value>)> {
    PermissionService::assert_admin(&user)?;
    let moderator = ModerationService::add_moderator(forum_id, dto.user_id, &state.db).await?;
    ModerationService::log(user.id, "moderator_added", Some(forum_id), None, None, Some(dto.user_id), None, &state.db).await;
    Ok((StatusCode::CREATED, Json(json!({ "moderator": moderator }))))
}

pub async fn remove_moderator(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path((forum_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    ModerationService::remove_moderator(forum_id, user_id, &state.db).await?;
    ModerationService::log(user.id, "moderator_removed", Some(forum_id), None, None, Some(user_id), None, &state.db).await;
    Ok(StatusCode::NO_CONTENT)
}

// ── Approval queue ──────────────────────────────────────────────────────────

/// Which forums this caller may decide for: `None` means "all of them"
/// (platform administrator), otherwise the forums they moderate. Returning the
/// scope rather than a yes/no is what keeps a moderator of one board from
/// releasing messages posted in another.
async fn moderation_scope(user: &ForumUser, state: &AppState) -> Result<Option<Vec<Uuid>>> {
    if user.is_admin() {
        return Ok(None);
    }
    let forums: Vec<Uuid> = sqlx::query_scalar(
        "SELECT forum_id FROM forum.moderators WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    if forums.is_empty() {
        return Err(ForumError::Forbidden);
    }
    Ok(Some(forums))
}

/// GET /mod/queue — contributions waiting for a decision.
pub async fn pending_queue(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let scope = moderation_scope(&user, &state).await?;
    let pending = ModerationService::list_pending(scope.as_deref(), 100, &state.db).await?;
    let total = ModerationService::count_pending(scope.as_deref(), &state.db).await?;
    Ok(Json(json!({ "pending": pending, "total": total })))
}

/// Refuses a decision on a message posted in a forum the caller does not moderate.
async fn assert_may_decide(post_id: Uuid, user: &ForumUser, state: &AppState) -> Result<()> {
    let scope = moderation_scope(user, state).await?;
    let Some(forums) = scope else { return Ok(()) }; // administrator: every forum
    let post = PostService::get(post_id, &state.db).await?;
    if forums.contains(&post.forum_id) { Ok(()) } else { Err(ForumError::Forbidden) }
}

/// POST /mod/queue/:id/approve — publish a held message (and its topic).
pub async fn approve_pending(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_may_decide(id, &user, &state).await?;
    ModerationService::approve_post(id, user.id, &state.db).await?;
    // Only now is the message public, so only now is it worth announcing.
    publisher::publish_post_created(&state, id, user.id).await;
    // Tell the author their held contribution was published.
    if let Ok(post) = PostService::get(id, &state.db).await {
        NotificationService::notify(&state, post.author_id, "approved", user.id, post.topic_id, Some(id), None).await;
    }
    Ok(Json(json!({ "ok": true })))
}

/// POST /mod/queue/:id/reject — discard a held message (and its topic).
pub async fn reject_pending(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    assert_may_decide(id, &user, &state).await?;
    // Capture the author before the post is gone, so the rejection can be told.
    let author = PostService::get(id, &state.db).await.ok().map(|p| p.author_id);
    ModerationService::reject_post(id, user.id, &state.db).await?;
    if let Some(author) = author {
        NotificationService::notify_simple(&state, author, "rejected", user.id, None).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

// ── Moderation log ──────────────────────────────────────────────────────────

pub async fn mod_log(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let scope = moderation_scope(&user, &state).await?;
    let entries = ModerationService::list_log(100, scope.as_deref(), user.id, &state.db).await?;
    Ok(Json(json!({ "log": entries })))
}

// ── Warnings ────────────────────────────────────────────────────────────────

pub async fn warn_user(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
    Json(dto): Json<WarnDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    assert_can_moderate(&user, &state).await?;
    let warning = ModerationService::warn(uid, user.id, &dto.reason, &state.db).await?;
    // The warned member should know — and know why.
    NotificationService::notify_simple(&state, uid, "warning", user.id, Some(&dto.reason)).await;
    Ok(Json(json!({ "warning": warning })))
}

pub async fn list_warnings(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_can_moderate(&user, &state).await?;
    let warnings = ModerationService::list_warnings(uid, &state.db).await?;
    Ok(Json(json!({ "warnings": warnings })))
}

// ── Bans (admin only) ───────────────────────────────────────────────────────

pub async fn ban_user(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
    Json(dto): Json<BanDto>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let ban = ModerationService::ban(uid, user.id, dto.reason.as_deref(), dto.days, &state.db).await?;
    // Tell the banned member, without leaking the moderators' internal reason.
    NotificationService::notify_simple(&state, uid, "ban", user.id, None).await;
    Ok(Json(json!({ "ban": ban })))
}

pub async fn unban_user(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    ModerationService::unban(uid, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_bans(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    assert_can_moderate(&user, &state).await?;
    let bans = ModerationService::list_bans(&state.db).await?;
    Ok(Json(json!({ "bans": bans })))
}

// ── IP bans (admin only, exact match) ──────────────────────────

pub async fn list_ip_bans(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let bans = ModerationService::list_ip_bans(&state.db).await?;
    Ok(Json(json!({ "bans": bans })))
}

pub async fn ban_ip(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<IpBanDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    PermissionService::assert_admin(&user)?;
    let ban = ModerationService::ban_ip(&dto.value, user.id, dto.reason.as_deref(), dto.days, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "ban": ban }))))
}

pub async fn unban_ip(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    ModerationService::unban_ip(id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Email bans (admin only, exact match) ───────────────────────

pub async fn list_email_bans(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let bans = ModerationService::list_email_bans(&state.db).await?;
    Ok(Json(json!({ "bans": bans })))
}

pub async fn ban_email(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<EmailBanDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    PermissionService::assert_admin(&user)?;
    let ban = ModerationService::ban_email(&dto.email, user.id, dto.reason.as_deref(), dto.days, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "ban": ban }))))
}

pub async fn unban_email(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    ModerationService::unban_email(id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Private notes ───────────────────────────────────────────────────────────

pub async fn add_note(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<ModNoteDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    assert_can_moderate(&user, &state).await?;
    let note = ModerationService::add_note(user.id, dto.target_user_id, dto.topic_id, dto.post_id, &dto.body, &state.db).await?;
    Ok(Json(json!({ "note": note })))
}

pub async fn list_notes(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_can_moderate(&user, &state).await?;
    let notes = ModerationService::list_notes(uid, &state.db).await?;
    Ok(Json(json!({ "notes": notes })))
}

// ── Soft delete / restore ───────────────────────────────────────────────────

async fn assert_can_moderate_post(post_id: Uuid, user: &ForumUser, state: &AppState) -> Result<()> {
    let post = PostService::get(post_id, &state.db).await?;
    let perms = PermissionService::effective(post.forum_id, user, &state.db).await?;
    if perms.is_admin || perms.is_moderator { Ok(()) } else { Err(ForumError::Forbidden) }
}

pub async fn remove_post(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    assert_can_moderate_post(id, &user, &state).await?;
    ModerationService::soft_delete_post(id, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_post(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    assert_can_moderate_post(id, &user, &state).await?;
    ModerationService::restore_post(id, user.id, &state.db).await?;
    Ok(Json(json!({ "ok": true })))
}

// ── Trash (soft-deleted topics) ─────────────────────────────────────────────

/// GET /trash — soft-deleted topics. Mods see their own forums, admins see all
/// (same scoping as the report queue and the log).
pub async fn list_trash(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let scope = moderation_scope(&user, &state).await?;
    let topics = TopicService::list_deleted(scope.as_deref(), 100, &state.db).await?;
    Ok(Json(json!({ "topics": topics })))
}

/// A moderator of the topic's own forum, or an admin — like the other
/// topic-moderation actions in handlers/topics.rs, but this one lives here
/// since it operates on already-deleted topics (the trash).
async fn assert_can_moderate_topic(topic_id: Uuid, user: &ForumUser, state: &AppState) -> Result<Uuid> {
    let topic = TopicService::get(topic_id, &state.db).await?;
    let perms = PermissionService::effective(topic.forum_id, user, &state.db).await?;
    if perms.is_admin || perms.is_moderator {
        Ok(topic.forum_id)
    } else {
        Err(ForumError::Forbidden)
    }
}

/// Body of `POST /mod/topics/bulk`: a batch of topics to lock/unlock/delete
/// together, e.g. from a moderator's multi-select in the topic list.
#[derive(Debug, Deserialize)]
pub struct BulkTopicsDto {
    pub topic_ids: Vec<Uuid>,
    pub action: String,
}

/// Largest batch a single call accepts. Keeps the request — and the sequential
/// per-topic work it triggers — bounded, rather than letting a client stream
/// an unbounded id list into one handler invocation.
const BULK_TOPICS_MAX: usize = 100;

/// POST /mod/topics/bulk — apply lock/unlock/delete to many topics at once.
///
/// Each topic is revalidated on its own (SEC-05): the batch may freely mix
/// topics from several forums, and a caller who moderates forum A must not be
/// able to smuggle an action onto a topic living in forum B just because its
/// id rode along in the same request. A topic that fails that check — or the
/// action itself — is silently skipped and counted, rather than aborting the
/// whole batch, so one bad id can't block the rest.
pub async fn bulk_topics(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<BulkTopicsDto>,
) -> Result<Json<Value>> {
    if dto.topic_ids.is_empty() {
        return Err(ForumError::Validation("no topics selected".into()));
    }
    if dto.topic_ids.len() > BULK_TOPICS_MAX {
        return Err(ForumError::Validation(format!(
            "too many topics in one batch ({BULK_TOPICS_MAX} max)"
        )));
    }
    if !matches!(dto.action.as_str(), "lock" | "unlock" | "delete") {
        return Err(ForumError::Validation(format!("invalid action: {}", dto.action)));
    }

    let mut done = 0i64;
    let mut skipped = 0i64;
    for &id in &dto.topic_ids {
        // Per-topic permission check — this is not just an optimization, it is
        // the boundary that keeps a mod of one forum off another forum's topics.
        let forum_id = match assert_can_moderate_topic(id, &user, &state).await {
            Ok(forum_id) => forum_id,
            Err(_) => { skipped += 1; continue; }
        };
        let (outcome, log_action) = match dto.action.as_str() {
            "lock"   => (TopicService::set_locked(id, true, &state.db).await.map(|_| ()), "lock_topic"),
            "unlock" => (TopicService::set_locked(id, false, &state.db).await.map(|_| ()), "unlock_topic"),
            _        => (TopicService::delete(id, &user, &state.db).await, "delete_topic"),
        };
        match outcome {
            Ok(()) => {
                done += 1;
                ModerationService::log(user.id, log_action, Some(forum_id), Some(id), None, None, None, &state.db).await;
            }
            Err(_) => skipped += 1,
        }
    }
    Ok(Json(json!({ "done": done, "skipped": skipped })))
}

/// POST /topics/:id/restore — bring a soft-deleted topic back. Any moderator
/// of its forum (or an admin) may do this, same as the soft-delete itself.
pub async fn restore_topic(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let forum_id = assert_can_moderate_topic(id, &user, &state).await?;
    let topic = TopicService::restore(id, &state.db).await?;
    ModerationService::log(user.id, "restore_topic", Some(forum_id), Some(id), None, None, None, &state.db).await;
    Ok(Json(json!({ "topic": topic })))
}

/// DELETE /topics/:id/purge — erase a soft-deleted topic for good. Admin only:
/// unlike restore, this is not reversible, so it is not left to ordinary
/// moderators the way the soft-delete/restore pair is.
pub async fn purge_topic(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    // Capture the forum before the topic is gone, so the purge leaves an audit trace.
    let topic = TopicService::get(id, &state.db).await?;
    TopicService::purge(id, &state.db).await?;
    ModerationService::log(user.id, "purge_topic", Some(topic.forum_id), Some(id), None, None, None, &state.db).await;
    Ok(StatusCode::NO_CONTENT)
}

// ── Prune (bulk soft-delete of stale topics) ────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PruneForumDto {
    pub days: i32,
    #[serde(default)]
    pub dry_run: bool,
}

/// POST /forums/:id/prune — admin only. `dry_run: true` only counts how many
/// topics would be affected (nothing is written), so the console can show a
/// preview before the admin commits to the actual soft-delete pass.
pub async fn prune_forum(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(forum_id): Path<Uuid>,
    Json(dto): Json<PruneForumDto>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let count = TopicService::prune_forum(forum_id, dto.days, dto.dry_run, user.id, &state.db).await?;
    if !dto.dry_run {
        ModerationService::log(
            user.id, "prune_forum", Some(forum_id), None, None, None,
            Some(&format!("{count} topic(s), older_than_days={}", dto.days)),
            &state.db,
        ).await;
    }
    Ok(Json(json!({ "count": count })))
}
