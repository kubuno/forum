//! Private messages: 1:1 / small-group conversations, entirely separate from
//! the discussion boards (see migration `000013`).
//!
//! SECURITY (the whole point of this subsystem): every read is filtered by
//! `forum.pm_participants` membership. A caller who is not a participant of a
//! thread gets [`ForumError::NotFound`] — never `Forbidden`, never a byte of
//! the thread's content — so probing a thread id reveals nothing about
//! whether it exists.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config::instance::InstanceConfig,
    errors::{ForumError, Result},
    models::pm::{CreateThreadDto, PmMessage, PmThread, ThreadDetail, ThreadSummary},
    services::directory,
    state::AppState,
};

/// How much of the latest message is shown in the thread list.
const PREVIEW_CHARS: usize = 140;
/// Defensive ceiling on how many messages a single `get_thread` call returns.
/// Not part of the contract (no pagination was asked for) — just a guard
/// against one enormous response for a thread nobody ever cleans up.
const MAX_MESSAGES: i64 = 1000;

pub struct PmService;

#[derive(sqlx::FromRow)]
struct ThreadSummaryRow {
    id:                Uuid,
    subject:           Option<String>,
    last_message_at:   DateTime<Utc>,
    participant_ids:   Option<Vec<Uuid>>,
    last_body:         Option<String>,
    unread_count:      i64,
}

impl PmService {
    /// SECURITY: the single gate every other read/write in this module goes
    /// through. Not a participant (or a participant who has hidden the
    /// thread) → 404, indistinguishable from a thread that never existed.
    async fn assert_participant(thread_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<()> {
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM forum.pm_participants
                            WHERE thread_id = $1 AND user_id = $2 AND deleted = FALSE)",
        )
        .bind(thread_id)
        .bind(user_id)
        .fetch_one(db)
        .await?;
        if !ok {
            return Err(ForumError::NotFound(format!("PM thread {thread_id}")));
        }
        Ok(())
    }

    /// Refuses when any of `targets` has blocked `sender`. Checked before a
    /// thread is created and before every message is sent — never only at
    /// creation time, since a block can happen mid-conversation.
    async fn assert_not_blocked(sender: Uuid, targets: &[Uuid], db: &PgPool) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let blocked: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM forum.pm_blocks
                            WHERE blocked_user_id = $1 AND user_id = ANY($2))",
        )
        .bind(sender)
        .bind(targets)
        .fetch_one(db)
        .await?;
        if blocked {
            // Generic on purpose (SEC): the sender must not learn *which*
            // recipient blocked them, only that the send didn't go through.
            return Err(ForumError::Forbidden);
        }
        Ok(())
    }

    /// Same anti-flood patron as `PostService::assert_not_flooding`, applied
    /// to the sender's most recent PM instead of their most recent post.
    /// Platform admins are exempt, same reasoning as moderators are for posts.
    async fn assert_not_flooding(
        sender: Uuid,
        is_admin: bool,
        cfg: &InstanceConfig,
        db: &PgPool,
    ) -> Result<()> {
        if is_admin || cfg.min_seconds_between_posts <= 0 {
            return Ok(());
        }
        let last: Option<DateTime<Utc>> =
            sqlx::query_scalar("SELECT MAX(created_at) FROM forum.pm_messages WHERE sender_id = $1")
                .bind(sender)
                .fetch_one(db)
                .await?;
        let Some(last) = last else { return Ok(()) };
        let elapsed = (Utc::now() - last).num_seconds();
        if elapsed < cfg.min_seconds_between_posts {
            return Err(ForumError::RateLimited(cfg.min_seconds_between_posts - elapsed));
        }
        Ok(())
    }

    /// Resolves recipient ids through the core's directory. Fails on the
    /// FIRST id that doesn't resolve, with a message that never names which
    /// one — a caller must not be able to use this to enumerate accounts.
    async fn assert_recipients_exist(state: &AppState, ids: &[Uuid]) -> Result<()> {
        for id in ids {
            if directory::display_name(state, *id).await.is_none() {
                return Err(ForumError::Validation("destinataire introuvable".into()));
            }
        }
        Ok(())
    }

    /// Creates a thread with its first message, atomically. `dto.recipient_ids`
    /// is deduplicated and stripped of the sender's own id before the quota,
    /// blocklist and directory checks run.
    pub async fn create_thread(
        state: &AppState,
        sender: Uuid,
        is_admin: bool,
        dto: CreateThreadDto,
        cfg: &InstanceConfig,
    ) -> Result<(PmThread, PmMessage)> {
        let mut recipients: Vec<Uuid> = dto.recipient_ids.into_iter().filter(|id| *id != sender).collect();
        recipients.sort();
        recipients.dedup();
        if recipients.is_empty() {
            return Err(ForumError::Validation("aucun destinataire valide".into()));
        }

        Self::assert_not_flooding(sender, is_admin, cfg, &state.db).await?;
        Self::assert_recipients_exist(state, &recipients).await?;
        Self::assert_not_blocked(sender, &recipients, &state.db).await?;

        let mut tx = state.db.begin().await?;

        let thread = sqlx::query_as::<_, PmThread>(
            "INSERT INTO forum.pm_threads (subject, created_by) VALUES ($1, $2) RETURNING *",
        )
        .bind(&dto.subject)
        .bind(sender)
        .fetch_one(&mut *tx)
        .await?;

        // The sender's own participant row starts "read" (they just wrote the
        // first message); everyone else starts unread (`last_read_at` NULL).
        sqlx::query(
            "INSERT INTO forum.pm_participants (thread_id, user_id, last_read_at)
             VALUES ($1, $2, NOW())",
        )
        .bind(thread.id)
        .bind(sender)
        .execute(&mut *tx)
        .await?;
        for uid in &recipients {
            sqlx::query("INSERT INTO forum.pm_participants (thread_id, user_id) VALUES ($1, $2)")
                .bind(thread.id)
                .bind(*uid)
                .execute(&mut *tx)
                .await?;
        }

        let message = sqlx::query_as::<_, PmMessage>(
            "INSERT INTO forum.pm_messages (thread_id, sender_id, body_md)
             VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(thread.id)
        .bind(sender)
        .bind(&dto.body_md)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        for recipient in &recipients {
            crate::services::notification_service::NotificationService::notify_pm(
                state, *recipient, sender, thread.id,
            )
            .await;
        }

        Ok((thread, message))
    }

    /// Appends a message to an existing thread. `sender` must already be a
    /// (non-hidden) participant — see [`Self::assert_participant`].
    pub async fn send_message(
        state: &AppState,
        thread_id: Uuid,
        sender: Uuid,
        is_admin: bool,
        body_md: String,
        cfg: &InstanceConfig,
    ) -> Result<PmMessage> {
        Self::assert_participant(thread_id, sender, &state.db).await?;
        Self::assert_not_flooding(sender, is_admin, cfg, &state.db).await?;

        let others: Vec<Uuid> = sqlx::query_scalar(
            "SELECT user_id FROM forum.pm_participants WHERE thread_id = $1 AND user_id <> $2",
        )
        .bind(thread_id)
        .bind(sender)
        .fetch_all(&state.db)
        .await?;
        Self::assert_not_blocked(sender, &others, &state.db).await?;

        let mut tx = state.db.begin().await?;

        let message = sqlx::query_as::<_, PmMessage>(
            "INSERT INTO forum.pm_messages (thread_id, sender_id, body_md)
             VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(thread_id)
        .bind(sender)
        .bind(&body_md)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE forum.pm_threads SET last_message_at = NOW() WHERE id = $1")
            .bind(thread_id)
            .execute(&mut *tx)
            .await?;
        // The sender's own message is never "unread" for them.
        sqlx::query(
            "UPDATE forum.pm_participants SET last_read_at = NOW()
              WHERE thread_id = $1 AND user_id = $2",
        )
        .bind(thread_id)
        .bind(sender)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        for recipient in &others {
            crate::services::notification_service::NotificationService::notify_pm(
                state, *recipient, sender, thread_id,
            )
            .await;
        }

        Ok(message)
    }

    /// `GET /me/pm` rows, most recently active thread first.
    pub async fn list_threads(user_id: Uuid, limit: i64, offset: i64, db: &PgPool) -> Result<Vec<ThreadSummary>> {
        let rows = sqlx::query_as::<_, ThreadSummaryRow>(
            "SELECT
                t.id, t.subject, t.last_message_at,
                (SELECT array_agg(pp.user_id) FROM forum.pm_participants pp
                  WHERE pp.thread_id = t.id) AS participant_ids,
                (SELECT m.body_md FROM forum.pm_messages m
                  WHERE m.thread_id = t.id ORDER BY m.created_at DESC LIMIT 1) AS last_body,
                (SELECT COUNT(*) FROM forum.pm_messages m
                  WHERE m.thread_id = t.id AND m.sender_id <> $1
                    AND m.created_at > COALESCE(p.last_read_at, '-infinity'::timestamptz)) AS unread_count
             FROM forum.pm_threads t
             JOIN forum.pm_participants p ON p.thread_id = t.id AND p.user_id = $1 AND p.deleted = FALSE
             ORDER BY t.last_message_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ThreadSummary {
                id:                   r.id,
                subject:              r.subject,
                participant_ids:      r.participant_ids.unwrap_or_default(),
                last_message_at:      r.last_message_at,
                last_message_preview: r.last_body.map(|b| Self::truncate(&b)),
                unread_count:         r.unread_count,
            })
            .collect())
    }

    /// Number of threads with at least one unread message for this user —
    /// the bell/badge count.
    pub async fn unread_count(user_id: Uuid, db: &PgPool) -> Result<i64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM forum.pm_participants p
              WHERE p.user_id = $1 AND p.deleted = FALSE
                AND EXISTS (SELECT 1 FROM forum.pm_messages m
                             WHERE m.thread_id = p.thread_id AND m.sender_id <> $1
                               AND m.created_at > COALESCE(p.last_read_at, '-infinity'::timestamptz))",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(n)
    }

    /// `GET /me/pm/:id` — 404 for a non-participant (SEC), never the content.
    pub async fn get_thread(thread_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<(ThreadDetail, Vec<PmMessage>)> {
        Self::assert_participant(thread_id, user_id, db).await?;

        let thread = sqlx::query_as::<_, PmThread>("SELECT * FROM forum.pm_threads WHERE id = $1")
            .bind(thread_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("PM thread {thread_id}")))?;

        let participant_ids: Vec<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM forum.pm_participants WHERE thread_id = $1")
                .bind(thread_id)
                .fetch_all(db)
                .await?;

        let messages = sqlx::query_as::<_, PmMessage>(
            "SELECT * FROM forum.pm_messages WHERE thread_id = $1 ORDER BY created_at ASC LIMIT $2",
        )
        .bind(thread_id)
        .bind(MAX_MESSAGES)
        .fetch_all(db)
        .await?;

        Ok((
            ThreadDetail { id: thread.id, subject: thread.subject, participant_ids },
            messages,
        ))
    }

    /// `POST /me/pm/:id/read`.
    pub async fn mark_read(thread_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<()> {
        Self::assert_participant(thread_id, user_id, db).await?;
        sqlx::query(
            "UPDATE forum.pm_participants SET last_read_at = NOW()
              WHERE thread_id = $1 AND user_id = $2",
        )
        .bind(thread_id)
        .bind(user_id)
        .execute(db)
        .await?;
        Ok(())
    }

    /// `DELETE /me/pm/:id` — hides the thread for this participant only.
    /// Once every participant has hidden it, the thread (and its messages)
    /// are dropped for good, in the same transaction as the last hide.
    pub async fn delete_for_user(thread_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<()> {
        Self::assert_participant(thread_id, user_id, db).await?;

        let mut tx = db.begin().await?;
        sqlx::query(
            "UPDATE forum.pm_participants SET deleted = TRUE
              WHERE thread_id = $1 AND user_id = $2",
        )
        .bind(thread_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM forum.pm_participants WHERE thread_id = $1 AND deleted = FALSE",
        )
        .bind(thread_id)
        .fetch_one(&mut *tx)
        .await?;
        if remaining == 0 {
            sqlx::query("DELETE FROM forum.pm_threads WHERE id = $1")
                .bind(thread_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// `POST /me/pm/blocks`.
    pub async fn block(user_id: Uuid, target: Uuid, db: &PgPool) -> Result<()> {
        if target == user_id {
            return Err(ForumError::Validation("impossible de se bloquer soi-même".into()));
        }
        sqlx::query(
            "INSERT INTO forum.pm_blocks (user_id, blocked_user_id) VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(target)
        .execute(db)
        .await?;
        Ok(())
    }

    /// `DELETE /me/pm/blocks/:uid`.
    pub async fn unblock(user_id: Uuid, target: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM forum.pm_blocks WHERE user_id = $1 AND blocked_user_id = $2")
            .bind(user_id)
            .bind(target)
            .execute(db)
            .await?;
        Ok(())
    }

    /// `GET /me/pm/blocks`.
    pub async fn list_blocks(user_id: Uuid, db: &PgPool) -> Result<Vec<Uuid>> {
        let rows: Vec<Uuid> = sqlx::query_scalar(
            "SELECT blocked_user_id FROM forum.pm_blocks WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Truncates a message body to [`PREVIEW_CHARS`] on a `char` boundary
    /// (never a byte boundary — the body is arbitrary UTF-8 Markdown).
    fn truncate(body: &str) -> String {
        let mut chars = body.chars();
        let head: String = chars.by_ref().take(PREVIEW_CHARS).collect();
        if chars.next().is_some() { format!("{head}…") } else { head }
    }
}
