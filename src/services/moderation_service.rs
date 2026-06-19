use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::moderation::{
        Ban, CreateReportDto, ModLogEntry, ModNote, Moderator, Report, ResolveReportDto, Warning,
    },
};

pub struct ModerationService;

impl ModerationService {
    // ── Reports ─────────────────────────────────────────────────────────────────

    pub async fn report_post(post_id: Uuid, reporter_id: Uuid, dto: CreateReportDto, db: &PgPool) -> Result<Report> {
        // Ensure the post exists (and surface a clean 404 otherwise).
        let exists: Option<i32> = sqlx::query_scalar("SELECT 1 FROM forum.posts WHERE id = $1")
            .bind(post_id)
            .fetch_optional(db)
            .await?;
        if exists.is_none() {
            return Err(ForumError::NotFound(format!("Post {post_id}")));
        }
        let row = sqlx::query_as::<_, Report>(
            "INSERT INTO forum.reports (post_id, reporter_id, reason)
             VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(post_id)
        .bind(reporter_id)
        .bind(&dto.reason)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn list_reports(status: Option<String>, db: &PgPool) -> Result<Vec<Report>> {
        let rows = match status {
            Some(s) => sqlx::query_as::<_, Report>(
                "SELECT * FROM forum.reports WHERE status = $1 ORDER BY created_at DESC",
            )
            .bind(s)
            .fetch_all(db)
            .await?,
            None => sqlx::query_as::<_, Report>(
                "SELECT * FROM forum.reports ORDER BY created_at DESC",
            )
            .fetch_all(db)
            .await?,
        };
        Ok(rows)
    }

    pub async fn resolve_report(id: Uuid, handler_id: Uuid, dto: ResolveReportDto, db: &PgPool) -> Result<Report> {
        if !matches!(dto.status.as_str(), "resolved" | "rejected") {
            return Err(ForumError::Validation(format!("invalid status: {}", dto.status)));
        }
        sqlx::query_as::<_, Report>(
            "UPDATE forum.reports SET status = $2, handled_by = $3, handled_at = NOW()
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(&dto.status)
        .bind(handler_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ForumError::NotFound(format!("Report {id}")))
    }

    // ── Moderators (per forum) ────────────────────────────────────────────────

    pub async fn list_moderators(forum_id: Uuid, db: &PgPool) -> Result<Vec<Moderator>> {
        let rows = sqlx::query_as::<_, Moderator>(
            "SELECT * FROM forum.moderators WHERE forum_id = $1 ORDER BY created_at",
        )
        .bind(forum_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn add_moderator(forum_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Moderator> {
        let row = sqlx::query_as::<_, Moderator>(
            "INSERT INTO forum.moderators (forum_id, user_id) VALUES ($1, $2)
             ON CONFLICT (forum_id, user_id) DO UPDATE SET forum_id = EXCLUDED.forum_id
             RETURNING *",
        )
        .bind(forum_id)
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn remove_moderator(forum_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM forum.moderators WHERE forum_id = $1 AND user_id = $2")
            .bind(forum_id)
            .bind(user_id)
            .execute(db)
            .await?;
        Ok(())
    }

    // ── Moderation log (audit trail) ──────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn log(
        moderator_id: Uuid,
        action: &str,
        forum_id: Option<Uuid>,
        topic_id: Option<Uuid>,
        post_id: Option<Uuid>,
        target_user_id: Option<Uuid>,
        details: Option<&str>,
        db: &PgPool,
    ) {
        if let Err(e) = sqlx::query(
            "INSERT INTO forum.mod_log (moderator_id, action, forum_id, topic_id, post_id, target_user_id, details)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(moderator_id)
        .bind(action)
        .bind(forum_id)
        .bind(topic_id)
        .bind(post_id)
        .bind(target_user_id)
        .bind(details)
        .execute(db)
        .await
        {
            tracing::warn!(error = %e, "mod_log insert failed");
        }
    }

    pub async fn list_log(limit: i64, db: &PgPool) -> Result<Vec<ModLogEntry>> {
        let rows = sqlx::query_as::<_, ModLogEntry>(
            "SELECT * FROM forum.mod_log ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit.clamp(1, 200))
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    // ── Warnings ──────────────────────────────────────────────────────────────

    pub async fn warn(user_id: Uuid, moderator_id: Uuid, reason: &str, db: &PgPool) -> Result<Warning> {
        let w = sqlx::query_as::<_, Warning>(
            "INSERT INTO forum.user_warnings (user_id, moderator_id, reason) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(user_id)
        .bind(moderator_id)
        .bind(reason)
        .fetch_one(db)
        .await?;
        Self::log(moderator_id, "warn", None, None, None, Some(user_id), Some(reason), db).await;
        Ok(w)
    }

    pub async fn list_warnings(user_id: Uuid, db: &PgPool) -> Result<Vec<Warning>> {
        let rows = sqlx::query_as::<_, Warning>(
            "SELECT * FROM forum.user_warnings WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    // ── Bans (forum-wide) ─────────────────────────────────────────────────────

    pub async fn ban(user_id: Uuid, by: Uuid, reason: Option<&str>, days: Option<i64>, db: &PgPool) -> Result<Ban> {
        let until = days.map(|d| chrono::Utc::now() + chrono::Duration::days(d));
        let b = sqlx::query_as::<_, Ban>(
            "INSERT INTO forum.user_bans (user_id, banned_by, reason, until) VALUES ($1, $2, $3, $4)
             ON CONFLICT (user_id) DO UPDATE SET banned_by = EXCLUDED.banned_by, reason = EXCLUDED.reason, until = EXCLUDED.until, created_at = NOW()
             RETURNING *",
        )
        .bind(user_id)
        .bind(by)
        .bind(reason)
        .bind(until)
        .fetch_one(db)
        .await?;
        Self::log(by, "ban", None, None, None, Some(user_id), reason, db).await;
        Ok(b)
    }

    pub async fn unban(user_id: Uuid, by: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM forum.user_bans WHERE user_id = $1")
            .bind(user_id)
            .execute(db)
            .await?;
        Self::log(by, "unban", None, None, None, Some(user_id), None, db).await;
        Ok(())
    }

    pub async fn list_bans(db: &PgPool) -> Result<Vec<Ban>> {
        let rows = sqlx::query_as::<_, Ban>("SELECT * FROM forum.user_bans ORDER BY created_at DESC")
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    /// Returns true if the user is currently banned (respecting expiry).
    pub async fn is_banned(user_id: Uuid, db: &PgPool) -> Result<bool> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM forum.user_bans WHERE user_id = $1 AND (until IS NULL OR until > NOW())",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(n > 0)
    }

    // ── Private moderator notes ────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn add_note(
        author_id: Uuid,
        target_user_id: Option<Uuid>,
        topic_id: Option<Uuid>,
        post_id: Option<Uuid>,
        body: &str,
        db: &PgPool,
    ) -> Result<ModNote> {
        let n = sqlx::query_as::<_, ModNote>(
            "INSERT INTO forum.mod_notes (author_id, target_user_id, topic_id, post_id, body)
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(author_id)
        .bind(target_user_id)
        .bind(topic_id)
        .bind(post_id)
        .bind(body)
        .fetch_one(db)
        .await?;
        Ok(n)
    }

    pub async fn list_notes(target_user_id: Uuid, db: &PgPool) -> Result<Vec<ModNote>> {
        let rows = sqlx::query_as::<_, ModNote>(
            "SELECT * FROM forum.mod_notes WHERE target_user_id = $1 ORDER BY created_at DESC LIMIT 100",
        )
        .bind(target_user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    // ── Soft delete / restore posts ───────────────────────────────────────────

    pub async fn soft_delete_post(post_id: Uuid, by: Uuid, db: &PgPool) -> Result<()> {
        let r = sqlx::query(
            "UPDATE forum.posts SET is_deleted = TRUE, deleted_at = NOW(), deleted_by = $2
             WHERE id = $1 AND is_deleted = FALSE",
        )
        .bind(post_id)
        .bind(by)
        .execute(db)
        .await?;
        if r.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("Post {post_id}")));
        }
        Self::log(by, "delete_post", None, None, Some(post_id), None, None, db).await;
        Ok(())
    }

    pub async fn restore_post(post_id: Uuid, by: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query(
            "UPDATE forum.posts SET is_deleted = FALSE, deleted_at = NULL, deleted_by = NULL WHERE id = $1",
        )
        .bind(post_id)
        .execute(db)
        .await?;
        Self::log(by, "restore_post", None, None, Some(post_id), None, None, db).await;
        Ok(())
    }
}
