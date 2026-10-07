use chrono::{Duration, Utc};
use kubuno_db::dialect::{Assign, SqlType};
use kubuno_db::{new_id, params, DbPool, DbQueryBuilder};
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

    pub async fn report_post(post_id: Uuid, reporter_id: Uuid, dto: CreateReportDto, db: &DbPool) -> Result<Report> {
        let b = db.backend();
        // Ensure the post exists (and surface a clean 404 otherwise).
        let exists: Option<i64> = db
            .fetch_optional_scalar(
                &format!("SELECT {} FROM forum.posts WHERE id = $1", b.cast("1", SqlType::BigInt)),
                params![post_id],
            )
            .await?;
        if exists.is_none() {
            return Err(ForumError::NotFound(format!("Post {post_id}")));
        }
        // SEC-15: one open report per (post, reporter). The former partial unique
        // index (`... WHERE status = 'open'`) has no portable spelling, so the
        // guard is a pre-insert check here instead.
        let dup: Option<i64> = db
            .fetch_optional_scalar(
                &format!(
                    "SELECT {} FROM forum.reports \
                      WHERE post_id = $1 AND reporter_id = $2 AND status = 'open' LIMIT 1",
                    b.cast("1", SqlType::BigInt)
                ),
                params![post_id, reporter_id],
            )
            .await?;
        if dup.is_some() {
            return Err(ForumError::Conflict("you have already reported this post".into()));
        }
        let id = new_id();
        db.execute(
            "INSERT INTO forum.reports (id, post_id, reporter_id, reason, reason_id) \
             VALUES ($1, $2, $3, $4, $5)",
            params![id, post_id, reporter_id, dto.reason, dto.reason_id],
        )
        .await?;
        db.fetch_one_as::<Report>("SELECT * FROM forum.reports WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    // ── Predefined report reasons (admin-curated chip list) ──────────────────────

    pub async fn list_report_reasons(db: &DbPool) -> Result<Vec<ReportReason>> {
        let rows = db
            .fetch_all_as::<ReportReason>(
                "SELECT * FROM forum.report_reasons ORDER BY position, created_at",
                params![],
            )
            .await?;
        Ok(rows)
    }

    pub async fn create_report_reason(dto: CreateReportReasonDto, db: &DbPool) -> Result<ReportReason> {
        let id = new_id();
        let description = dto.description.as_deref().map(str::trim).filter(|s| !s.is_empty());
        db.execute(
            "INSERT INTO forum.report_reasons (id, title, description, position) \
             VALUES ($1, $2, $3, $4)",
            params![id, dto.title.trim(), description, dto.position],
        )
        .await?;
        db.fetch_one_as::<ReportReason>("SELECT * FROM forum.report_reasons WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn delete_report_reason(id: Uuid, db: &DbPool) -> Result<()> {
        let affected = db
            .execute("DELETE FROM forum.report_reasons WHERE id = $1", params![id])
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Report reason {id}")));
        }
        Ok(())
    }

    /// Lists reports. `forum_ids = None` means every forum (administrators);
    /// otherwise only reports on posts in the forums the caller moderates (SEC-06).
    pub async fn list_reports(
        status: Option<String>,
        forum_ids: Option<&[Uuid]>,
        db: &DbPool,
    ) -> Result<Vec<Report>> {
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "SELECT r.* FROM forum.reports r JOIN forum.posts p ON p.id = r.post_id WHERE TRUE",
        );
        if let Some(s) = &status {
            qb.push(" AND r.status = ").push_bind(s.clone());
        }
        if let Some(ids) = forum_ids {
            qb.push(" AND p.forum_id").push_in(ids.iter().copied());
        }
        qb.push(" ORDER BY r.created_at DESC");
        let rows = qb.fetch_all_as::<Report>(db).await?;
        Ok(rows)
    }

    pub async fn resolve_report(id: Uuid, handler_id: Uuid, dto: ResolveReportDto, db: &DbPool) -> Result<Report> {
        if !matches!(dto.status.as_str(), "resolved" | "rejected") {
            return Err(ForumError::Validation(format!("invalid status: {}", dto.status)));
        }
        let affected = db
            .execute(
                "UPDATE forum.reports SET status = $1, handled_by = $2, handled_at = $3 WHERE id = $4",
                params![dto.status, handler_id, Utc::now(), id],
            )
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Report {id}")));
        }
        db.fetch_one_as::<Report>("SELECT * FROM forum.reports WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    // ── Moderators (per forum) ────────────────────────────────────────────────

    pub async fn list_moderators(forum_id: Uuid, db: &DbPool) -> Result<Vec<Moderator>> {
        let rows = db
            .fetch_all_as::<Moderator>(
                "SELECT * FROM forum.moderators WHERE forum_id = $1 ORDER BY created_at",
                params![forum_id],
            )
            .await?;
        Ok(rows)
    }

    pub async fn add_moderator(forum_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Moderator> {
        let b = db.backend();
        // moderators has PK (forum_id, user_id); an existing row is a no-op.
        let sql = format!(
            "INSERT {}INTO forum.moderators (forum_id, user_id) VALUES ($1, $2){}",
            b.insert_ignore_prefix(),
            b.on_conflict_do_nothing(&["forum_id", "user_id"])
        );
        db.execute(&sql, params![forum_id, user_id]).await?;
        db.fetch_one_as::<Moderator>(
            "SELECT * FROM forum.moderators WHERE forum_id = $1 AND user_id = $2",
            params![forum_id, user_id],
        )
        .await
        .map_err(Into::into)
    }

    pub async fn remove_moderator(forum_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        db.execute(
            "DELETE FROM forum.moderators WHERE forum_id = $1 AND user_id = $2",
            params![forum_id, user_id],
        )
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
        db: &DbPool,
    ) {
        // mod_log.id is a BIGINT autoincrement — no minted uuid here.
        if let Err(e) = db
            .execute(
                "INSERT INTO forum.mod_log (moderator_id, action, forum_id, topic_id, post_id, target_user_id, details) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                params![
                    moderator_id,
                    action,
                    forum_id,
                    topic_id,
                    post_id,
                    target_user_id,
                    details.map(|s| s.to_string())
                ],
            )
            .await
        {
            tracing::warn!(error = %e, "mod_log insert failed");
        }
    }

    /// Lists the moderation log. `forum_ids = None` is every forum (admins);
    /// otherwise entries for the caller's forums, plus their own actions (SEC-06).
    pub async fn list_log(
        limit: i64,
        forum_ids: Option<&[Uuid]>,
        self_id: Uuid,
        db: &DbPool,
    ) -> Result<Vec<ModLogEntry>> {
        let mut qb = DbQueryBuilder::new(db.backend(), "SELECT * FROM forum.mod_log");
        if let Some(ids) = forum_ids {
            qb.push(" WHERE (forum_id")
                .push_in(ids.iter().copied())
                .push(" OR moderator_id = ")
                .push_bind(self_id)
                .push(")");
        }
        qb.push(" ORDER BY created_at DESC LIMIT ").push_bind(limit.clamp(1, 200));
        let rows = qb.fetch_all_as::<ModLogEntry>(db).await?;
        Ok(rows)
    }

    // ── Warnings ──────────────────────────────────────────────────────────────

    pub async fn warn(user_id: Uuid, moderator_id: Uuid, reason: &str, db: &DbPool) -> Result<Warning> {
        let id = new_id();
        db.execute(
            "INSERT INTO forum.user_warnings (id, user_id, moderator_id, reason) VALUES ($1, $2, $3, $4)",
            params![id, user_id, moderator_id, reason],
        )
        .await?;
        let w = db
            .fetch_one_as::<Warning>("SELECT * FROM forum.user_warnings WHERE id = $1", params![id])
            .await?;
        Self::log(moderator_id, "warn", None, None, None, Some(user_id), Some(reason), db).await;
        Ok(w)
    }

    pub async fn list_warnings(user_id: Uuid, db: &DbPool) -> Result<Vec<Warning>> {
        let rows = db
            .fetch_all_as::<Warning>(
                "SELECT * FROM forum.user_warnings WHERE user_id = $1 ORDER BY created_at DESC",
                params![user_id],
            )
            .await?;
        Ok(rows)
    }

    // ── Bans (forum-wide) ─────────────────────────────────────────────────────

    pub async fn ban(user_id: Uuid, by: Uuid, reason: Option<&str>, days: Option<i64>, db: &DbPool) -> Result<Ban> {
        if user_id == by {
            return Err(ForumError::Validation("you cannot ban yourself".into()));
        }
        let until = days.map(|d| Utc::now() + Duration::days(d));
        let now = Utc::now();
        let b = db.backend();
        let sql = format!(
            "INSERT INTO forum.user_bans (user_id, banned_by, reason, until, created_at) \
             VALUES ($1, $2, $3, $4, $5){}",
            b.upsert(
                "user_bans",
                &["user_id"],
                &[
                    Assign::Incoming("banned_by"),
                    Assign::Incoming("reason"),
                    Assign::Incoming("until"),
                    Assign::Incoming("created_at"),
                ],
            )
        );
        db.execute(&sql, params![user_id, by, reason.map(str::to_string), until, now]).await?;
        let ban = db
            .fetch_one_as::<Ban>("SELECT * FROM forum.user_bans WHERE user_id = $1", params![user_id])
            .await?;
        Self::log(by, "ban", None, None, None, Some(user_id), reason, db).await;
        BanRegistry::reload(db).await?;
        Ok(ban)
    }

    pub async fn unban(user_id: Uuid, by: Uuid, db: &DbPool) -> Result<()> {
        db.execute("DELETE FROM forum.user_bans WHERE user_id = $1", params![user_id]).await?;
        Self::log(by, "unban", None, None, None, Some(user_id), None, db).await;
        BanRegistry::reload(db).await?;
        Ok(())
    }

    pub async fn list_bans(db: &DbPool) -> Result<Vec<Ban>> {
        let rows = db
            .fetch_all_as::<Ban>("SELECT * FROM forum.user_bans ORDER BY created_at DESC", params![])
            .await?;
        Ok(rows)
    }

    /// Rejects a write from a currently-banned user (defense-in-depth alongside
    /// `middleware::enforce_ban`).
    pub async fn assert_not_banned(user_id: Uuid, db: &DbPool) -> Result<()> {
        if Self::is_banned(user_id, db).await? {
            return Err(ForumError::Forbidden);
        }
        Ok(())
    }

    /// Returns true if the user is currently banned, read from the in-memory
    /// `BanRegistry` — no database round trip.
    pub async fn is_banned(user_id: Uuid, _db: &DbPool) -> Result<bool> {
        Ok(BanRegistry::is_user_banned(user_id))
    }

    // ── IP bans (admin only, exact match) ────────────────────────

    pub async fn ban_ip(value: &str, by: Uuid, reason: Option<&str>, days: Option<i64>, db: &DbPool) -> Result<IpBan> {
        let ip: std::net::IpAddr = value
            .trim()
            .parse()
            .map_err(|_| ForumError::Validation(format!("invalid IP address: {value}")))?;
        let canonical = ip.to_string();
        let until = days.map(|d| Utc::now() + Duration::days(d));
        let now = Utc::now();
        let b = db.backend();
        let sql = format!(
            "INSERT INTO forum.ip_bans (id, value, banned_by, reason, until, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6){}",
            b.upsert(
                "ip_bans",
                &["value"],
                &[
                    Assign::Incoming("banned_by"),
                    Assign::Incoming("reason"),
                    Assign::Incoming("until"),
                    Assign::Incoming("created_at"),
                ],
            )
        );
        db.execute(&sql, params![new_id(), canonical.clone(), by, reason.map(str::to_string), until, now])
            .await
            .inspect_err(|e| tracing::error!(error = %e, "Banning an IP address"))?;
        let ban = db
            .fetch_one_as::<IpBan>("SELECT * FROM forum.ip_bans WHERE value = $1", params![canonical.clone()])
            .await?;
        Self::log(by, "ban_ip", None, None, None, None, Some(&canonical), db).await;
        BanRegistry::reload(db).await?;
        Ok(ban)
    }

    pub async fn unban_ip(id: Uuid, db: &DbPool) -> Result<()> {
        let affected = db
            .execute("DELETE FROM forum.ip_bans WHERE id = $1", params![id])
            .await
            .inspect_err(|e| tracing::error!(error = %e, %id, "Removing an IP ban"))?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("IP ban {id}")));
        }
        BanRegistry::reload(db).await?;
        Ok(())
    }

    pub async fn list_ip_bans(db: &DbPool) -> Result<Vec<IpBan>> {
        let rows = db
            .fetch_all_as::<IpBan>("SELECT * FROM forum.ip_bans ORDER BY created_at DESC", params![])
            .await
            .inspect_err(|e| tracing::error!(error = %e, "Listing IP bans"))?;
        Ok(rows)
    }

    // ── Email bans (admin only, exact match) ─────────────────────

    pub async fn ban_email(email: &str, by: Uuid, reason: Option<&str>, days: Option<i64>, db: &DbPool) -> Result<EmailBan> {
        let normalized = email.trim().to_lowercase();
        if normalized.is_empty() || !normalized.contains('@') {
            return Err(ForumError::Validation(format!("invalid email address: {email}")));
        }
        let until = days.map(|d| Utc::now() + Duration::days(d));
        let now = Utc::now();
        let b = db.backend();
        let sql = format!(
            "INSERT INTO forum.email_bans (id, email, banned_by, reason, until, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6){}",
            b.upsert(
                "email_bans",
                &["email"],
                &[
                    Assign::Incoming("banned_by"),
                    Assign::Incoming("reason"),
                    Assign::Incoming("until"),
                    Assign::Incoming("created_at"),
                ],
            )
        );
        db.execute(&sql, params![new_id(), normalized.clone(), by, reason.map(str::to_string), until, now])
            .await
            .inspect_err(|e| tracing::error!(error = %e, "Banning an email address"))?;
        let ban = db
            .fetch_one_as::<EmailBan>("SELECT * FROM forum.email_bans WHERE email = $1", params![normalized.clone()])
            .await?;
        Self::log(by, "ban_email", None, None, None, None, Some(&normalized), db).await;
        BanRegistry::reload(db).await?;
        Ok(ban)
    }

    pub async fn unban_email(id: Uuid, db: &DbPool) -> Result<()> {
        let affected = db
            .execute("DELETE FROM forum.email_bans WHERE id = $1", params![id])
            .await
            .inspect_err(|e| tracing::error!(error = %e, %id, "Removing an email ban"))?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Email ban {id}")));
        }
        BanRegistry::reload(db).await?;
        Ok(())
    }

    pub async fn list_email_bans(db: &DbPool) -> Result<Vec<EmailBan>> {
        let rows = db
            .fetch_all_as::<EmailBan>("SELECT * FROM forum.email_bans ORDER BY created_at DESC", params![])
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
        db: &DbPool,
    ) -> Result<ModNote> {
        let id = new_id();
        db.execute(
            "INSERT INTO forum.mod_notes (id, author_id, target_user_id, topic_id, post_id, body) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            params![id, author_id, target_user_id, topic_id, post_id, body],
        )
        .await?;
        db.fetch_one_as::<ModNote>("SELECT * FROM forum.mod_notes WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn list_notes(target_user_id: Uuid, db: &DbPool) -> Result<Vec<ModNote>> {
        let rows = db
            .fetch_all_as::<ModNote>(
                "SELECT * FROM forum.mod_notes WHERE target_user_id = $1 ORDER BY created_at DESC LIMIT 100",
                params![target_user_id],
            )
            .await?;
        Ok(rows)
    }

    // ── Approval queue ────────────────────────────────────────────────────────

    /// Messages waiting for a decision, newest last so a moderator works through
    /// them in the order they were written.
    pub async fn list_pending(
        forum_ids: Option<&[Uuid]>,
        limit: i64,
        db: &DbPool,
    ) -> Result<Vec<PendingPost>> {
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "SELECT p.id, p.topic_id, p.forum_id, p.author_id, p.body_md, \
                    p.is_first_post, p.created_at, \
                    t.title AS topic_title, f.name AS forum_name \
               FROM forum.posts p \
               JOIN forum.topics t ON t.id = p.topic_id \
               JOIN forum.forums f ON f.id = p.forum_id \
              WHERE p.is_approved = FALSE AND p.is_deleted = FALSE",
        );
        if let Some(ids) = forum_ids {
            qb.push(" AND p.forum_id").push_in(ids.iter().copied());
        }
        qb.push(" ORDER BY p.created_at LIMIT ").push_bind(limit.clamp(1, 200));
        let rows = qb.fetch_all_as::<PendingPost>(db).await?;
        Ok(rows)
    }

    /// How many messages are waiting, for the badge on the moderation entry.
    pub async fn count_pending(forum_ids: Option<&[Uuid]>, db: &DbPool) -> Result<i64> {
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "SELECT COUNT(*) FROM forum.posts WHERE is_approved = FALSE AND is_deleted = FALSE",
        );
        if let Some(ids) = forum_ids {
            qb.push(" AND forum_id").push_in(ids.iter().copied());
        }
        let n: i64 = qb.fetch_scalar(db).await?;
        Ok(n)
    }

    /// Publishes a held message. If it opens a topic, the topic is published
    /// with it.
    pub async fn approve_post(post_id: Uuid, by: Uuid, db: &DbPool) -> Result<()> {
        let mut tx = db.begin().await?;

        // No `UPDATE ... RETURNING` (MySQL): read the pending row, then update it.
        let row = tx
            .fetch_optional_row(
                "SELECT topic_id, forum_id, author_id, is_first_post FROM forum.posts \
                  WHERE id = $1 AND is_approved = FALSE",
                params![post_id],
            )
            .await
            .inspect_err(|e| tracing::error!(error = %e, %post_id, "Approving a pending post"))?;
        let Some(row) = row else {
            tx.rollback().await?;
            return Err(ForumError::NotFound(format!("Pending post {post_id}")));
        };
        let topic_id = row.try_get::<Uuid>("topic_id")?;
        let forum_id = row.try_get::<Uuid>("forum_id")?;
        let author_id = row.try_get::<Uuid>("author_id")?;
        let is_first = row.try_get::<bool>("is_first_post")?;

        tx.execute(
            "UPDATE forum.posts SET is_approved = TRUE, approved_at = $1, approved_by = $2 WHERE id = $3",
            params![Utc::now(), by, post_id],
        )
        .await?;

        if is_first {
            tx.execute(
                "UPDATE forum.topics SET is_approved = TRUE, approved_at = $1, approved_by = $2 \
                  WHERE id = $3 AND is_approved = FALSE",
                params![Utc::now(), by, topic_id],
            )
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

    /// Discards a held message (removed outright, never soft-deleted).
    pub async fn reject_post(post_id: Uuid, by: Uuid, db: &DbPool) -> Result<()> {
        let mut tx = db.begin().await?;

        // No `DELETE ... RETURNING` (MySQL): read the pending row, then delete it.
        let row = tx
            .fetch_optional_row(
                "SELECT topic_id, forum_id, author_id, is_first_post FROM forum.posts \
                  WHERE id = $1 AND is_approved = FALSE",
                params![post_id],
            )
            .await
            .inspect_err(|e| tracing::error!(error = %e, %post_id, "Rejecting a pending post"))?;
        let Some(row) = row else {
            tx.rollback().await?;
            return Err(ForumError::NotFound(format!("Pending post {post_id}")));
        };
        let topic_id = row.try_get::<Uuid>("topic_id")?;
        let forum_id = row.try_get::<Uuid>("forum_id")?;
        let author_id = row.try_get::<Uuid>("author_id")?;
        let is_first = row.try_get::<bool>("is_first_post")?;

        tx.execute("DELETE FROM forum.posts WHERE id = $1", params![post_id]).await?;

        if is_first {
            // Cascade takes the remaining posts of the topic with it.
            tx.execute("DELETE FROM forum.topics WHERE id = $1", params![topic_id]).await?;
        } else {
            aggregates::recompute_topic(&mut tx, topic_id).await?;
        }
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        tx.commit().await?;

        Self::log(by, "reject_post", Some(forum_id), None, None, Some(author_id), None, db).await;
        Ok(())
    }

    /// Discards contributions that have waited longer than the instance allows.
    pub async fn purge_stale_pending(days: i32, db: &DbPool) -> Result<u64> {
        if days <= 0 {
            return Ok(0);
        }
        // `NOW() - make_interval(...)` is computed in Rust and bound as a cutoff.
        let cutoff = Utc::now() - Duration::days(days as i64);
        // Topics first: deleting a topic cascades to its posts.
        let topics = db
            .execute(
                "DELETE FROM forum.topics WHERE is_approved = FALSE AND created_at < $1",
                params![cutoff],
            )
            .await
            .inspect_err(|e| tracing::error!(error = %e, "Purging stale pending topics"))?;

        let posts = db
            .execute(
                "DELETE FROM forum.posts WHERE is_approved = FALSE AND created_at < $1",
                params![cutoff],
            )
            .await
            .inspect_err(|e| tracing::error!(error = %e, "Purging stale pending posts"))?;

        Ok(topics + posts)
    }

    // ── Soft delete / restore posts ───────────────────────────────────────────

    pub async fn soft_delete_post(post_id: Uuid, by: Uuid, db: &DbPool) -> Result<()> {
        let affected = db
            .execute(
                "UPDATE forum.posts SET is_deleted = TRUE, deleted_at = $1, deleted_by = $2 \
                 WHERE id = $3 AND is_deleted = FALSE",
                params![Utc::now(), by, post_id],
            )
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Post {post_id}")));
        }
        Self::log(by, "delete_post", None, None, Some(post_id), None, None, db).await;
        Ok(())
    }

    pub async fn restore_post(post_id: Uuid, by: Uuid, db: &DbPool) -> Result<()> {
        db.execute(
            "UPDATE forum.posts SET is_deleted = FALSE, deleted_at = NULL, deleted_by = NULL WHERE id = $1",
            params![post_id],
        )
        .await?;
        Self::log(by, "restore_post", None, None, Some(post_id), None, None, db).await;
        Ok(())
    }
}
