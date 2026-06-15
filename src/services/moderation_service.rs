use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::moderation::{CreateReportDto, Moderator, Report, ResolveReportDto},
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
}
