//! Private messages: 1:1 / small-group conversations, entirely separate from
//! the discussion boards (see migration `000013`).
//!
//! SECURITY (the whole point of this subsystem): every read is filtered by
//! `forum.pm_participants` membership. A caller who is not a participant of a
//! thread gets [`ForumError::NotFound`] — never `Forbidden`, never a byte of
//! the thread's content — so probing a thread id reveals nothing about
//! whether it exists.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use kubuno_db::dialect::SqlType;
use kubuno_db::{new_id, params, DbPool, DbQueryBuilder};
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
const MAX_MESSAGES: i64 = 1000;

pub struct PmService;

#[derive(sqlx::FromRow)]
struct ThreadSummaryRow {
    id:              Uuid,
    subject:         Option<String>,
    last_message_at: DateTime<Utc>,
    last_body:       Option<String>,
    unread_count:    i64,
}

impl PmService {
    /// SECURITY: the single gate every other read/write in this module goes
    /// through. Not a participant (or a participant who has hidden the
    /// thread) → 404, indistinguishable from a thread that never existed.
    async fn assert_participant(thread_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        let one = db.backend().cast("1", SqlType::BigInt);
        let hit: Option<i64> = db
            .fetch_optional_scalar(
                &format!(
                    "SELECT {one} FROM forum.pm_participants \
                      WHERE thread_id = $1 AND user_id = $2 AND deleted = FALSE LIMIT 1"
                ),
                params![thread_id, user_id],
            )
            .await?;
        if hit.is_none() {
            return Err(ForumError::NotFound(format!("PM thread {thread_id}")));
        }
        Ok(())
    }

    /// Refuses when any of `targets` has blocked `sender`. Checked before a
    /// thread is created and before every message is sent — never only at
    /// creation time, since a block can happen mid-conversation.
    async fn assert_not_blocked(sender: Uuid, targets: &[Uuid], db: &DbPool) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let one = db.backend().cast("1", SqlType::BigInt);
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            format!("SELECT {one} FROM forum.pm_blocks WHERE blocked_user_id = "),
        );
        qb.push_bind(sender).push(" AND user_id").push_in(targets.iter().copied()).push(" LIMIT 1");
        let blocked: Option<i64> = qb.fetch_optional_scalar(db).await?;
        if blocked.is_some() {
            // Generic on purpose (SEC): the sender must not learn *which*
            // recipient blocked them, only that the send didn't go through.
            return Err(ForumError::Forbidden);
        }
        Ok(())
    }

    /// Same anti-flood patron as `PostService::assert_not_flooding`, applied
    /// to the sender's most recent PM instead of their most recent post.
    async fn assert_not_flooding(
        sender: Uuid,
        is_admin: bool,
        cfg: &InstanceConfig,
        db: &DbPool,
    ) -> Result<()> {
        if is_admin || cfg.min_seconds_between_posts <= 0 {
            return Ok(());
        }
        let last: Option<DateTime<Utc>> = db
            .fetch_scalar(
                "SELECT MAX(created_at) FROM forum.pm_messages WHERE sender_id = $1",
                params![sender],
            )
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

    /// Creates a thread with its first message, atomically.
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

        let thread_id = new_id();
        let message_id = new_id();
        let now = Utc::now();

        let mut tx = state.db.begin().await?;

        tx.execute(
            "INSERT INTO forum.pm_threads (id, subject, created_by) VALUES ($1, $2, $3)",
            params![thread_id, dto.subject, sender],
        )
        .await?;

        // The sender's own participant row starts "read" (they just wrote the
        // first message); everyone else starts unread (`last_read_at` NULL).
        tx.execute(
            "INSERT INTO forum.pm_participants (thread_id, user_id, last_read_at) VALUES ($1, $2, $3)",
            params![thread_id, sender, now],
        )
        .await?;
        for uid in &recipients {
            tx.execute(
                "INSERT INTO forum.pm_participants (thread_id, user_id) VALUES ($1, $2)",
                params![thread_id, *uid],
            )
            .await?;
        }

        tx.execute(
            "INSERT INTO forum.pm_messages (id, thread_id, sender_id, body_md) VALUES ($1, $2, $3, $4)",
            params![message_id, thread_id, sender, dto.body_md],
        )
        .await?;

        tx.commit().await?;

        let thread = Self::thread_row(thread_id, &state.db).await?;
        let message = Self::message_row(message_id, &state.db).await?;

        for recipient in &recipients {
            crate::services::notification_service::NotificationService::notify_pm(
                state, *recipient, sender, thread_id,
            )
            .await;
        }

        Ok((thread, message))
    }

    async fn thread_row(id: Uuid, db: &DbPool) -> Result<PmThread> {
        db.fetch_one_as::<PmThread>("SELECT * FROM forum.pm_threads WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    async fn message_row(id: Uuid, db: &DbPool) -> Result<PmMessage> {
        db.fetch_one_as::<PmMessage>("SELECT * FROM forum.pm_messages WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    /// Appends a message to an existing thread.
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

        let others: Vec<Uuid> = state
            .db
            .fetch_all_as::<(Uuid,)>(
                "SELECT user_id FROM forum.pm_participants WHERE thread_id = $1 AND user_id <> $2",
                params![thread_id, sender],
            )
            .await?
            .into_iter()
            .map(|(u,)| u)
            .collect();
        Self::assert_not_blocked(sender, &others, &state.db).await?;

        let message_id = new_id();
        let now = Utc::now();
        let mut tx = state.db.begin().await?;

        tx.execute(
            "INSERT INTO forum.pm_messages (id, thread_id, sender_id, body_md) VALUES ($1, $2, $3, $4)",
            params![message_id, thread_id, sender, body_md],
        )
        .await?;
        tx.execute(
            "UPDATE forum.pm_threads SET last_message_at = $1 WHERE id = $2",
            params![now, thread_id],
        )
        .await?;
        // The sender's own message is never "unread" for them.
        tx.execute(
            "UPDATE forum.pm_participants SET last_read_at = $1 WHERE thread_id = $2 AND user_id = $3",
            params![now, thread_id, sender],
        )
        .await?;

        tx.commit().await?;

        let message = Self::message_row(message_id, &state.db).await?;
        for recipient in &others {
            crate::services::notification_service::NotificationService::notify_pm(
                state, *recipient, sender, thread_id,
            )
            .await;
        }
        Ok(message)
    }

    /// `GET /me/pm` rows, most recently active thread first.
    pub async fn list_threads(user_id: Uuid, limit: i64, offset: i64, db: &DbPool) -> Result<Vec<ThreadSummary>> {
        // `array_agg` has no portable spelling: the participant ids are fetched
        // in a second grouped query and stitched in Rust. The unread count uses
        // `last_read_at IS NULL OR created_at > last_read_at` in place of the
        // PostgreSQL-only `COALESCE(..., '-infinity'::timestamptz)`.
        let rows = db
            .fetch_all_as::<ThreadSummaryRow>(
                "SELECT \
                    t.id, t.subject, t.last_message_at, \
                    (SELECT m.body_md FROM forum.pm_messages m \
                      WHERE m.thread_id = t.id ORDER BY m.created_at DESC LIMIT 1) AS last_body, \
                    (SELECT COUNT(*) FROM forum.pm_messages m \
                      WHERE m.thread_id = t.id AND m.sender_id <> $1 \
                        AND (p.last_read_at IS NULL OR m.created_at > p.last_read_at)) AS unread_count \
                 FROM forum.pm_threads t \
                 JOIN forum.pm_participants p ON p.thread_id = t.id AND p.user_id = $2 AND p.deleted = FALSE \
                 ORDER BY t.last_message_at DESC \
                 LIMIT $3 OFFSET $4",
                params![user_id, user_id, limit, offset],
            )
            .await?;

        let thread_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let mut participants: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        if !thread_ids.is_empty() {
            let mut qb = DbQueryBuilder::new(
                db.backend(),
                "SELECT thread_id, user_id FROM forum.pm_participants WHERE thread_id",
            );
            qb.push_in(thread_ids.iter().copied());
            let pairs: Vec<(Uuid, Uuid)> = qb.fetch_all_as(db).await?;
            for (tid, uid) in pairs {
                participants.entry(tid).or_default().push(uid);
            }
        }

        Ok(rows
            .into_iter()
            .map(|r| ThreadSummary {
                participant_ids:      participants.remove(&r.id).unwrap_or_default(),
                id:                   r.id,
                subject:              r.subject,
                last_message_at:      r.last_message_at,
                last_message_preview: r.last_body.map(|b| Self::truncate(&b)),
                unread_count:         r.unread_count,
            })
            .collect())
    }

    /// Number of threads with at least one unread message for this user.
    pub async fn unread_count(user_id: Uuid, db: &DbPool) -> Result<i64> {
        let n: i64 = db
            .fetch_scalar(
                "SELECT COUNT(*) FROM forum.pm_participants p \
                  WHERE p.user_id = $1 AND p.deleted = FALSE \
                    AND EXISTS (SELECT 1 FROM forum.pm_messages m \
                                 WHERE m.thread_id = p.thread_id AND m.sender_id <> $2 \
                                   AND (p.last_read_at IS NULL OR m.created_at > p.last_read_at))",
                params![user_id, user_id],
            )
            .await?;
        Ok(n)
    }

    /// `GET /me/pm/:id` — 404 for a non-participant (SEC), never the content.
    pub async fn get_thread(thread_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<(ThreadDetail, Vec<PmMessage>)> {
        Self::assert_participant(thread_id, user_id, db).await?;

        let thread = db
            .fetch_optional_as::<PmThread>("SELECT * FROM forum.pm_threads WHERE id = $1", params![thread_id])
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("PM thread {thread_id}")))?;

        let participant_ids: Vec<Uuid> = db
            .fetch_all_as::<(Uuid,)>(
                "SELECT user_id FROM forum.pm_participants WHERE thread_id = $1",
                params![thread_id],
            )
            .await?
            .into_iter()
            .map(|(u,)| u)
            .collect();

        let messages = db
            .fetch_all_as::<PmMessage>(
                "SELECT * FROM forum.pm_messages WHERE thread_id = $1 ORDER BY created_at ASC LIMIT $2",
                params![thread_id, MAX_MESSAGES],
            )
            .await?;

        Ok((
            ThreadDetail { id: thread.id, subject: thread.subject, participant_ids },
            messages,
        ))
    }

    /// `POST /me/pm/:id/read`.
    pub async fn mark_read(thread_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        Self::assert_participant(thread_id, user_id, db).await?;
        db.execute(
            "UPDATE forum.pm_participants SET last_read_at = $1 WHERE thread_id = $2 AND user_id = $3",
            params![Utc::now(), thread_id, user_id],
        )
        .await?;
        Ok(())
    }

    /// `DELETE /me/pm/:id` — hides the thread for this participant only.
    pub async fn delete_for_user(thread_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        Self::assert_participant(thread_id, user_id, db).await?;

        let mut tx = db.begin().await?;
        tx.execute(
            "UPDATE forum.pm_participants SET deleted = TRUE WHERE thread_id = $1 AND user_id = $2",
            params![thread_id, user_id],
        )
        .await?;

        let remaining: i64 = tx
            .fetch_optional_scalar(
                "SELECT COUNT(*) FROM forum.pm_participants WHERE thread_id = $1 AND deleted = FALSE",
                params![thread_id],
            )
            .await?
            .unwrap_or(0);
        if remaining == 0 {
            tx.execute("DELETE FROM forum.pm_threads WHERE id = $1", params![thread_id]).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// `POST /me/pm/blocks`.
    pub async fn block(user_id: Uuid, target: Uuid, db: &DbPool) -> Result<()> {
        if target == user_id {
            return Err(ForumError::Validation("impossible de se bloquer soi-même".into()));
        }
        let b = db.backend();
        let sql = format!(
            "INSERT {}INTO forum.pm_blocks (user_id, blocked_user_id) VALUES ($1, $2){}",
            b.insert_ignore_prefix(),
            b.on_conflict_do_nothing(&["user_id", "blocked_user_id"])
        );
        db.execute(&sql, params![user_id, target]).await?;
        Ok(())
    }

    /// `DELETE /me/pm/blocks/:uid`.
    pub async fn unblock(user_id: Uuid, target: Uuid, db: &DbPool) -> Result<()> {
        db.execute(
            "DELETE FROM forum.pm_blocks WHERE user_id = $1 AND blocked_user_id = $2",
            params![user_id, target],
        )
        .await?;
        Ok(())
    }

    /// `GET /me/pm/blocks`.
    pub async fn list_blocks(user_id: Uuid, db: &DbPool) -> Result<Vec<Uuid>> {
        let rows: Vec<Uuid> = db
            .fetch_all_as::<(Uuid,)>(
                "SELECT blocked_user_id FROM forum.pm_blocks WHERE user_id = $1 ORDER BY created_at DESC",
                params![user_id],
            )
            .await?
            .into_iter()
            .map(|(u,)| u)
            .collect();
        Ok(rows)
    }

    /// Truncates a message body to [`PREVIEW_CHARS`] on a `char` boundary.
    fn truncate(body: &str) -> String {
        let mut chars = body.chars();
        let head: String = chars.by_ref().take(PREVIEW_CHARS).collect();
        if chars.next().is_some() { format!("{head}…") } else { head }
    }
}
