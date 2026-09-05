use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::moderation::{
        Ban, CreateReportDto, CreateReportReasonDto, EmailBan, IpBan, ModLogEntry, ModNote,
        Moderator, PendingPost, Report, ReportReason, ResolveReportDto, Warning,
    },
    services::{aggregates, ban_registry::BanRegistry, rank_service::RankService},
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
        // SEC-15: one open report per (post, reporter). A repeat while the first
        // is still open is a no-op, reported back as a clean conflict rather than
        // piling identical rows onto the moderators' queue. 'open' is the actual
        // open state — the reports CHECK never allowed 'pending' (see 000009).
        let row = sqlx::query_as::<_, Report>(
            "INSERT INTO forum.reports (post_id, reporter_id, reason, reason_id)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (post_id, reporter_id) WHERE status = 'open' DO NOTHING
             RETURNING *",
        )
        .bind(post_id)
        .bind(reporter_id)
        .bind(&dto.reason)
        .bind(dto.reason_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ForumError::Conflict("you have already reported this post".into()))?;
        Ok(row)
    }

    // ── Predefined report reasons (admin-curated chip list) ──────────────────────

    pub async fn list_report_reasons(db: &PgPool) -> Result<Vec<ReportReason>> {
        let rows = sqlx::query_as::<_, ReportReason>(
            "SELECT * FROM forum.report_reasons ORDER BY position, created_at",
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn create_report_reason(dto: CreateReportReasonDto, db: &PgPool) -> Result<ReportReason> {
        let reason = sqlx::query_as::<_, ReportReason>(
            "INSERT INTO forum.report_reasons (title, description, position)
             VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(dto.title.trim())
        .bind(dto.description.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .bind(dto.position)
        .fetch_one(db)
        .await?;
        Ok(reason)
    }

    pub async fn delete_report_reason(id: Uuid, db: &PgPool) -> Result<()> {
        let r = sqlx::query("DELETE FROM forum.report_reasons WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        if r.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("Report reason {id}")));
        }
        Ok(())
    }

    /// Lists reports. `forum_ids = None` means every forum (administrators);
    /// otherwise only reports on posts in the forums the caller moderates, so a
    /// moderator of one board never sees another's queue (SEC-06).
    pub async fn list_reports(
        status: Option<String>,
        forum_ids: Option<&[Uuid]>,
        db: &PgPool,
    ) -> Result<Vec<Report>> {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT r.* FROM forum.reports r JOIN forum.posts p ON p.id = r.post_id WHERE TRUE",
        );
        if let Some(s) = &status {
            qb.push(" AND r.status = ").push_bind(s);
        }
        if let Some(ids) = forum_ids {
            qb.push(" AND p.forum_id = ANY(").push_bind(ids).push(")");
        }
        qb.push(" ORDER BY r.created_at DESC");
        let rows = qb.build_query_as::<Report>().fetch_all(db).await?;
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

    /// Lists the moderation log. `forum_ids = None` is every forum (admins);
    /// otherwise entries for the caller's forums, plus their own actions so a
    /// moderator can always audit what they themselves did (SEC-06).
    pub async fn list_log(
        limit: i64,
        forum_ids: Option<&[Uuid]>,
        self_id: Uuid,
        db: &PgPool,
    ) -> Result<Vec<ModLogEntry>> {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM forum.mod_log");
        if let Some(ids) = forum_ids {
            qb.push(" WHERE (forum_id = ANY(")
                .push_bind(ids)
                .push(") OR moderator_id = ")
                .push_bind(self_id)
                .push(")");
        }
        qb.push(" ORDER BY created_at DESC LIMIT ").push_bind(limit.clamp(1, 200));
        let rows = qb.build_query_as::<ModLogEntry>().fetch_all(db).await?;
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
        // A moderator cannot lock themselves out (SEC-25). Protecting a platform
        // founder/super-admin from being banned needs the core to say who that is,
        // and is left for the group work that exposes such attributes to modules.
        if user_id == by {
            return Err(ForumError::Validation("you cannot ban yourself".into()));
        }
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
        BanRegistry::reload(db).await?;
        Ok(b)
    }

    pub async fn unban(user_id: Uuid, by: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM forum.user_bans WHERE user_id = $1")
            .bind(user_id)
            .execute(db)
            .await?;
        Self::log(by, "unban", None, None, None, Some(user_id), None, db).await;
        BanRegistry::reload(db).await?;
        Ok(())
    }

    pub async fn list_bans(db: &PgPool) -> Result<Vec<Ban>> {
        let rows = sqlx::query_as::<_, Ban>("SELECT * FROM forum.user_bans ORDER BY created_at DESC")
            .fetch_all(db)
            .await?;
        Ok(rows)
    }

    /// Rejects a write from a currently-banned user. For the public write paths
    /// (reacting, reporting, voting) that previously checked nothing, so a ban
    /// only stopped new posts and topics while everything else stayed open (SEC-13).
    /// Kept as defense-in-depth alongside `middleware::enforce_ban`, which is the
    /// primary enforcement point for every authenticated route.
    pub async fn assert_not_banned(user_id: Uuid, db: &PgPool) -> Result<()> {
        if Self::is_banned(user_id, db).await? {
            return Err(ForumError::Forbidden);
        }
        Ok(())
    }

    /// Returns true if the user is currently banned (respecting expiry), read
    /// from the in-memory `BanRegistry` — no database round trip. `db` is kept
    /// in the signature only for call-site compatibility.
    pub async fn is_banned(user_id: Uuid, _db: &PgPool) -> Result<bool> {
        Ok(BanRegistry::is_user_banned(user_id))
    }

    // ── IP bans (admin only, phpBB-style, exact match) ────────────────────────

    pub async fn ban_ip(value: &str, by: Uuid, reason: Option<&str>, days: Option<i64>, db: &PgPool) -> Result<IpBan> {
        let ip: std::net::IpAddr = value
            .trim()
            .parse()
            .map_err(|_| ForumError::Validation(format!("invalid IP address: {value}")))?;
        let canonical = ip.to_string();
        let until = days.map(|d| chrono::Utc::now() + chrono::Duration::days(d));
        let b = sqlx::query_as::<_, IpBan>(
            "INSERT INTO forum.ip_bans (value, banned_by, reason, until) VALUES ($1, $2, $3, $4)
             ON CONFLICT (value) DO UPDATE SET banned_by = EXCLUDED.banned_by, reason = EXCLUDED.reason, until = EXCLUDED.until, created_at = NOW()
             RETURNING *",
        )
        .bind(&canonical)
        .bind(by)
        .bind(reason)
        .bind(until)
        .fetch_one(db)
        .await
        .inspect_err(|e| tracing::error!(error = %e, "Banning an IP address"))?;
        Self::log(by, "ban_ip", None, None, None, None, Some(&canonical), db).await;
        BanRegistry::reload(db).await?;
        Ok(b)
    }

    pub async fn unban_ip(id: Uuid, db: &PgPool) -> Result<()> {
        let r = sqlx::query("DELETE FROM forum.ip_bans WHERE id = $1")
            .bind(id)
            .execute(db)
            .await
            .inspect_err(|e| tracing::error!(error = %e, %id, "Removing an IP ban"))?;
        if r.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("IP ban {id}")));
        }
        BanRegistry::reload(db).await?;
        Ok(())
    }

    pub async fn list_ip_bans(db: &PgPool) -> Result<Vec<IpBan>> {
        let rows = sqlx::query_as::<_, IpBan>("SELECT * FROM forum.ip_bans ORDER BY created_at DESC")
            .fetch_all(db)
            .await
            .inspect_err(|e| tracing::error!(error = %e, "Listing IP bans"))?;
        Ok(rows)
    }

    // ── Email bans (admin only, phpBB-style, exact match) ─────────────────────

    pub async fn ban_email(email: &str, by: Uuid, reason: Option<&str>, days: Option<i64>, db: &PgPool) -> Result<EmailBan> {
        let normalized = email.trim().to_lowercase();
        if normalized.is_empty() || !normalized.contains('@') {
            return Err(ForumError::Validation(format!("invalid email address: {email}")));
        }
        let until = days.map(|d| chrono::Utc::now() + chrono::Duration::days(d));
        let b = sqlx::query_as::<_, EmailBan>(
            "INSERT INTO forum.email_bans (email, banned_by, reason, until) VALUES ($1, $2, $3, $4)
             ON CONFLICT (email) DO UPDATE SET banned_by = EXCLUDED.banned_by, reason = EXCLUDED.reason, until = EXCLUDED.until, created_at = NOW()
             RETURNING *",
        )
        .bind(&normalized)
        .bind(by)
        .bind(reason)
        .bind(until)
        .fetch_one(db)
        .await
        .inspect_err(|e| tracing::error!(error = %e, "Banning an email address"))?;
        Self::log(by, "ban_email", None, None, None, None, Some(&normalized), db).await;
        BanRegistry::reload(db).await?;
        Ok(b)
    }

    pub async fn unban_email(id: Uuid, db: &PgPool) -> Result<()> {
        let r = sqlx::query("DELETE FROM forum.email_bans WHERE id = $1")
            .bind(id)
            .execute(db)
            .await
            .inspect_err(|e| tracing::error!(error = %e, %id, "Removing an email ban"))?;
        if r.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("Email ban {id}")));
        }
        BanRegistry::reload(db).await?;
        Ok(())
    }

    pub async fn list_email_bans(db: &PgPool) -> Result<Vec<EmailBan>> {
        let rows = sqlx::query_as::<_, EmailBan>("SELECT * FROM forum.email_bans ORDER BY created_at DESC")
            .fetch_all(db)
            .await
            .inspect_err(|e| tracing::error!(error = %e, "Listing email bans"))?;
        Ok(rows)
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

    // ── Approval queue ────────────────────────────────────────────────────────
    //
    // Contributions held by `forum.post_approval_mode` wait here. The queue is
    // over POSTS: a topic held back always has its opening message held with it,
    // so releasing that message releases the topic and discarding it discards
    // the topic. One decision, one place to make it.

    /// Messages waiting for a decision, newest last so a moderator works through
    /// them in the order they were written. `forum_ids = None` means "every
    /// forum" (platform administrators); otherwise only the forums the caller
    /// moderates.
    pub async fn list_pending(
        forum_ids: Option<&[Uuid]>,
        limit: i64,
        db: &PgPool,
    ) -> Result<Vec<PendingPost>> {
        let rows = match forum_ids {
            None => sqlx::query_as::<_, PendingPost>(
                "SELECT p.id, p.topic_id, p.forum_id, p.author_id, p.body_md,
                        p.is_first_post, p.created_at,
                        t.title AS topic_title, f.name AS forum_name
                   FROM forum.posts p
                   JOIN forum.topics t ON t.id = p.topic_id
                   JOIN forum.forums f ON f.id = p.forum_id
                  WHERE p.is_approved = FALSE AND p.is_deleted = FALSE
                  ORDER BY p.created_at LIMIT $1",
            )
            .bind(limit.clamp(1, 200))
            .fetch_all(db)
            .await?,
            Some(ids) => sqlx::query_as::<_, PendingPost>(
                "SELECT p.id, p.topic_id, p.forum_id, p.author_id, p.body_md,
                        p.is_first_post, p.created_at,
                        t.title AS topic_title, f.name AS forum_name
                   FROM forum.posts p
                   JOIN forum.topics t ON t.id = p.topic_id
                   JOIN forum.forums f ON f.id = p.forum_id
                  WHERE p.is_approved = FALSE AND p.is_deleted = FALSE
                    AND p.forum_id = ANY($1)
                  ORDER BY p.created_at LIMIT $2",
            )
            .bind(ids)
            .bind(limit.clamp(1, 200))
            .fetch_all(db)
            .await?,
        };
        Ok(rows)
    }

    /// How many messages are waiting, for the badge on the moderation entry.
    pub async fn count_pending(forum_ids: Option<&[Uuid]>, db: &PgPool) -> Result<i64> {
        let n: i64 = match forum_ids {
            None => sqlx::query_scalar(
                "SELECT COUNT(*) FROM forum.posts WHERE is_approved = FALSE AND is_deleted = FALSE",
            )
            .fetch_one(db)
            .await?,
            Some(ids) => sqlx::query_scalar(
                "SELECT COUNT(*) FROM forum.posts
                  WHERE is_approved = FALSE AND is_deleted = FALSE AND forum_id = ANY($1)",
            )
            .bind(ids)
            .fetch_one(db)
            .await?,
        };
        Ok(n)
    }

    /// Publishes a held message. If it opens a topic, the topic is published
    /// with it. The author's counters are bumped HERE rather than at creation,
    /// so a queued message never counts towards the "new member" threshold that
    /// decides whether the next one is queued too.
    pub async fn approve_post(post_id: Uuid, by: Uuid, db: &PgPool) -> Result<()> {
        let mut tx = db.begin().await?;

        let row = sqlx::query_as::<_, (Uuid, Uuid, Uuid, bool)>(
            "UPDATE forum.posts SET is_approved = TRUE, approved_at = NOW(), approved_by = $2
              WHERE id = $1 AND is_approved = FALSE
             RETURNING topic_id, forum_id, author_id, is_first_post",
        )
        .bind(post_id)
        .bind(by)
        .fetch_optional(&mut *tx)
        .await
        .inspect_err(|e| tracing::error!(error = %e, %post_id, "Approving a pending post"))?;

        let Some((topic_id, forum_id, author_id, is_first)) = row else {
            // Already approved, or gone: nothing to do, and no reason to fail.
            tx.rollback().await?;
            return Err(ForumError::NotFound(format!("Pending post {post_id}")));
        };

        if is_first {
            sqlx::query(
                "UPDATE forum.topics SET is_approved = TRUE, approved_at = NOW(), approved_by = $2
                  WHERE id = $1 AND is_approved = FALSE",
            )
            .bind(topic_id)
            .bind(by)
            .execute(&mut *tx)
            .await?;
            RankService::bump_topic_count(&mut tx, author_id, 1).await?;
        }

        RankService::bump_post_count(&mut tx, author_id, 1).await?;
        aggregates::recompute_topic(&mut tx, topic_id).await?;
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        tx.commit().await?;

        Self::log(by, "approve_post", Some(forum_id), Some(topic_id), Some(post_id), Some(author_id), None, db).await;
        Ok(())
    }

    /// Discards a held message. A rejected contribution was never published, so
    /// it is removed outright rather than soft-deleted: soft deletion exists to
    /// keep a trace of something readers already saw, and nobody saw this.
    /// Rejecting a topic's opening message removes the topic with it.
    pub async fn reject_post(post_id: Uuid, by: Uuid, db: &PgPool) -> Result<()> {
        let mut tx = db.begin().await?;

        let row = sqlx::query_as::<_, (Uuid, Uuid, Uuid, bool)>(
            "DELETE FROM forum.posts WHERE id = $1 AND is_approved = FALSE
             RETURNING topic_id, forum_id, author_id, is_first_post",
        )
        .bind(post_id)
        .fetch_optional(&mut *tx)
        .await
        .inspect_err(|e| tracing::error!(error = %e, %post_id, "Rejecting a pending post"))?;

        let Some((topic_id, forum_id, author_id, is_first)) = row else {
            tx.rollback().await?;
            return Err(ForumError::NotFound(format!("Pending post {post_id}")));
        };

        if is_first {
            // Cascade takes the remaining posts of the topic with it.
            sqlx::query("DELETE FROM forum.topics WHERE id = $1")
                .bind(topic_id)
                .execute(&mut *tx)
                .await?;
        } else {
            aggregates::recompute_topic(&mut tx, topic_id).await?;
        }
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        tx.commit().await?;

        Self::log(by, "reject_post", Some(forum_id), None, None, Some(author_id), None, db).await;
        Ok(())
    }

    /// Discards contributions that have waited longer than the instance allows.
    /// Returns how many were removed. `days <= 0` disables the sweep entirely.
    pub async fn purge_stale_pending(days: i32, db: &PgPool) -> Result<u64> {
        if days <= 0 {
            return Ok(0);
        }
        // Topics first: deleting a topic cascades to its posts, so this order
        // never leaves a topic whose opening message has just been removed.
        // Neither statement disturbs the denormalised counters — those only ever
        // counted approved rows, so a pending row leaving changes nothing.
        let topics = sqlx::query(
            "DELETE FROM forum.topics
              WHERE is_approved = FALSE AND created_at < NOW() - make_interval(days => $1)",
        )
        .bind(days)
        .execute(db)
        .await
        .inspect_err(|e| tracing::error!(error = %e, "Purging stale pending topics"))?
        .rows_affected();

        let posts = sqlx::query(
            "DELETE FROM forum.posts
              WHERE is_approved = FALSE AND created_at < NOW() - make_interval(days => $1)",
        )
        .bind(days)
        .execute(db)
        .await
        .inspect_err(|e| tracing::error!(error = %e, "Purging stale pending posts"))?
        .rows_affected();

        Ok(topics + posts)
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
