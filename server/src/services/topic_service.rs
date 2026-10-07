use chrono::{DateTime, Duration, Utc};
use kubuno_db::dialect::SqlType;
use kubuno_db::search::normalize;
use kubuno_db::{new_id, params, DbPool, DbQueryBuilder};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::{
        post::Post,
        topic::{CreateTopicDto, MergeTopicDto, MoveTopicDto, SplitTopicDto, Topic, UpdateTopicDto},
    },
    services::{aggregates, permission_service::PermissionService, rank_service::RankService},
};

const VALID_TYPES: [&str; 4] = ["normal", "sticky", "announcement", "global"];

/// The WHERE predicate shared by `prune_forum`'s preview and its actual pass:
/// stale, non-deleted topics of one forum. Pinned types, a solved question, and
/// any topic carrying a poll are excluded — a prune is meant for dead ordinary
/// discussions, not content an admin (or a poll's voters) expects to keep
/// finding in place. The two placeholder tokens are the forum id and the
/// activity cutoff (`Utc::now() - older_than_days`, computed in Rust so no
/// engine-specific interval arithmetic reaches the SQL).
macro_rules! prune_predicate {
    ($forum:literal, $cutoff:literal) => {
        concat!(
            " forum_id = ", $forum, " AND is_deleted = FALSE
              AND COALESCE(last_post_at, created_at) < ", $cutoff,
            " AND topic_type NOT IN ('sticky', 'announcement', 'global')
              AND is_solved = FALSE
              AND NOT EXISTS (SELECT 1 FROM forum.polls WHERE topic_id = forum.topics.id) "
        )
    };
}

pub struct TopicService;

impl TopicService {
    /// Topics of a forum, as this viewer may see them. A topic waiting for
    /// approval stays visible to its own author (so they can tell it was
    /// received, not lost) and to moderators, and to nobody else.
    pub async fn list_by_forum(
        forum_id: Uuid,
        viewer_id: Uuid,
        is_moderator: bool,
        limit: i64,
        offset: i64,
        db: &DbPool,
    ) -> Result<Vec<Topic>> {
        // Pinned types first (global, announcement, sticky), then by latest
        // activity (`(x IS NULL), x DESC` is the portable form of `x DESC NULLS
        // LAST`: NULL rows sort last on all three engines).
        let rows = db
            .fetch_all_as::<Topic>(
                "SELECT * FROM forum.topics \
                  WHERE forum_id = $1 AND is_deleted = FALSE \
                    AND (is_approved = TRUE OR author_id = $2 OR $3) \
                 ORDER BY CASE topic_type \
                            WHEN 'global'       THEN 0 \
                            WHEN 'announcement' THEN 1 \
                            WHEN 'sticky'       THEN 2 \
                            ELSE 3 END, \
                          (last_post_at IS NULL), last_post_at DESC, created_at DESC \
                 LIMIT $4 OFFSET $5",
                params![forum_id, viewer_id, is_moderator, limit, offset],
            )
            .await?;
        Ok(rows)
    }

    pub async fn count_by_forum(
        forum_id: Uuid,
        viewer_id: Uuid,
        is_moderator: bool,
        db: &DbPool,
    ) -> Result<i64> {
        let n: i64 = db
            .fetch_scalar(
                "SELECT COUNT(*) FROM forum.topics \
                  WHERE forum_id = $1 AND is_deleted = FALSE \
                    AND (is_approved = TRUE OR author_id = $2 OR $3)",
                params![forum_id, viewer_id, is_moderator],
            )
            .await?;
        Ok(n)
    }

    pub async fn get(id: Uuid, db: &DbPool) -> Result<Topic> {
        db.fetch_optional_as::<Topic>("SELECT * FROM forum.topics WHERE id = $1", params![id])
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("Topic {id}")))
    }

    /// This user's read marker for one topic — `None` when no row exists yet.
    pub async fn read_marker(topic_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Option<DateTime<Utc>>> {
        let read_at: Option<DateTime<Utc>> = db
            .fetch_optional_scalar(
                "SELECT read_at FROM forum.read_markers WHERE topic_id = $1 AND user_id = $2",
                params![topic_id, user_id],
            )
            .await?;
        Ok(read_at)
    }

    /// Cross-forum discovery feed. `kind` ∈ recent | unanswered | popular |
    /// unread | mine. Hidden forums (user role can_view = false) are excluded.
    #[allow(clippy::too_many_arguments)]
    pub async fn feed(
        kind: &str,
        user_id: Uuid,
        solved: Option<bool>,
        tag_id: Option<Uuid>,
        limit: i64,
        offset: i64,
        db: &DbPool,
    ) -> Result<Vec<Topic>> {
        let mut qb = DbQueryBuilder::new(db.backend(), "SELECT t.* FROM forum.topics t ");
        Self::push_feed_where(&mut qb, kind, user_id, solved, tag_id);

        let order: &'static str = match kind {
            "popular" => "ORDER BY (t.view_count + t.reply_count * 3) DESC, (t.last_post_at IS NULL), t.last_post_at DESC ",
            "unanswered" | "mine" => "ORDER BY t.created_at DESC ",
            _ => "ORDER BY (t.last_post_at IS NULL), t.last_post_at DESC, t.created_at DESC ",
        };
        qb.push(order);
        qb.push_limit_offset(limit.clamp(1, 100), offset.max(0));

        let rows = qb.fetch_all_as::<Topic>(db).await?;
        Ok(rows)
    }

    /// Total topics matching `feed`'s filters — same WHERE clause, no
    /// ORDER/LIMIT, so pagination never counts a topic the caller cannot see.
    pub async fn feed_count(
        kind: &str,
        user_id: Uuid,
        solved: Option<bool>,
        tag_id: Option<Uuid>,
        db: &DbPool,
    ) -> Result<i64> {
        let mut qb = DbQueryBuilder::new(db.backend(), "SELECT COUNT(*) FROM forum.topics t ");
        Self::push_feed_where(&mut qb, kind, user_id, solved, tag_id);
        let n: i64 = qb.fetch_scalar(db).await?;
        Ok(n)
    }

    /// The WHERE clause shared by `feed` and `feed_count` (SEC-01: the count can
    /// never be less restrictive than the listing).
    fn push_feed_where(
        qb: &mut DbQueryBuilder,
        kind: &str,
        user_id: Uuid,
        solved: Option<bool>,
        tag_id: Option<Uuid>,
    ) {
        if kind == "unread" {
            qb.push("LEFT JOIN forum.read_markers rm ON rm.topic_id = t.id AND rm.user_id = ")
                .push_bind(user_id)
                .push(" ");
        }
        qb.push(
            "WHERE NOT EXISTS (SELECT 1 FROM forum.permissions pm \
                 WHERE pm.forum_id = t.forum_id AND pm.role = 'user' AND pm.can_view = FALSE) ",
        );
        // A soft-deleted topic is gone from discovery for everyone (SEC-12).
        qb.push("AND t.is_deleted = FALSE ");
        // Cross-forum discovery must not surface what moderation is still holding.
        qb.push("AND (t.is_approved = TRUE OR t.author_id = ").push_bind(user_id).push(") ");

        match kind {
            "unanswered" => { qb.push("AND t.reply_count = 0 "); }
            "mine" => { qb.push("AND t.author_id = ").push_bind(user_id).push(" "); }
            "unread" => { qb.push("AND (rm.user_id IS NULL OR t.last_post_at > rm.read_at) AND t.last_post_at IS NOT NULL "); }
            _ => {}
        }
        if let Some(s) = solved {
            qb.push("AND t.is_solved = ").push_bind(s).push(" ");
        }
        if let Some(tid) = tag_id {
            qb.push("AND EXISTS (SELECT 1 FROM forum.topic_tags tt WHERE tt.topic_id = t.id AND tt.tag_id = ")
                .push_bind(tid).push(") ");
        }
    }

    /// Topics authored by a given user (public profile listing).
    pub async fn by_author(author_id: Uuid, limit: i64, db: &DbPool) -> Result<Vec<Topic>> {
        // Public listing: approved only.
        let rows = db
            .fetch_all_as::<Topic>(
                "SELECT * FROM forum.topics WHERE author_id = $1 AND is_approved \
                 ORDER BY created_at DESC LIMIT $2",
                params![author_id, limit.clamp(1, 100)],
            )
            .await?;
        Ok(rows)
    }

    pub async fn touch_view(id: Uuid, db: &DbPool) -> Result<()> {
        db.execute(
            "UPDATE forum.topics SET view_count = view_count + 1 WHERE id = $1",
            params![id],
        )
        .await?;
        Ok(())
    }

    /// Create a topic together with its opening post.
    pub async fn create(
        forum_id: Uuid,
        author_id: Uuid,
        topic_type: &str,
        dto: CreateTopicDto,
        approved: bool,
        db: &DbPool,
    ) -> Result<(Topic, Post)> {
        if !VALID_TYPES.contains(&topic_type) {
            return Err(ForumError::Validation(format!("invalid topic type: {topic_type}")));
        }
        let slug = aggregates::slugify(&dto.title);
        let approved_at: Option<DateTime<Utc>> = if approved { Some(Utc::now()) } else { None };
        let topic_id = new_id();
        let post_id = new_id();
        let b = db.backend();
        let title_norm = normalize(&dto.title);
        let body_norm = normalize(&dto.body_md);

        let mut tx = db.begin().await?;
        tx.execute(
            "INSERT INTO forum.topics \
                (id, forum_id, author_id, title, slug, topic_type, is_question, prefix, \
                 is_approved, approved_at, title_norm) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            params![
                topic_id,
                forum_id,
                author_id,
                dto.title,
                slug,
                topic_type,
                dto.is_question,
                dto.prefix.as_deref().filter(|s| !s.is_empty()),
                approved,
                approved_at,
                title_norm
            ],
        )
        .await?;

        tx.execute(
            "INSERT INTO forum.posts \
                (id, topic_id, forum_id, author_id, body_md, is_first_post, is_approved, \
                 approved_at, body_norm) \
             VALUES ($1, $2, $3, $4, $5, TRUE, $6, $7, $8)",
            params![
                post_id,
                topic_id,
                forum_id,
                author_id,
                dto.body_md,
                approved,
                approved_at,
                body_norm
            ],
        )
        .await?;

        aggregates::recompute_topic(&mut tx, topic_id).await?;
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        if approved {
            RankService::bump_post_count(&mut tx, author_id, 1).await?;
            RankService::bump_topic_count(&mut tx, author_id, 1).await?;
        }

        // Attach selected tags (only ones that exist). `$2` is bound twice by two
        // distinct placeholders — a placeholder is never reused on the portable
        // path.
        let tag_sql = format!(
            "INSERT {}INTO forum.topic_tags (topic_id, tag_id) \
             SELECT $1, $2 WHERE EXISTS (SELECT {} FROM forum.tags WHERE id = $3){}",
            b.insert_ignore_prefix(),
            b.cast("1", SqlType::BigInt),
            b.on_conflict_do_nothing(&["topic_id", "tag_id"])
        );
        for tag_id in &dto.tag_ids {
            tx.execute(&tag_sql, params![topic_id, tag_id, tag_id]).await?;
        }

        // Attach an optional poll.
        if let Some(poll) = &dto.poll {
            let opts: Vec<String> = poll
                .options
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if opts.len() >= 2 {
                let poll_id = new_id();
                tx.execute(
                    "INSERT INTO forum.polls (id, topic_id, question, is_multiple, closes_at) \
                     VALUES ($1, $2, $3, $4, $5)",
                    params![poll_id, topic_id, poll.question.clone(), poll.is_multiple, poll.closes_at],
                )
                .await?;
                for (i, opt) in opts.iter().enumerate() {
                    tx.execute(
                        "INSERT INTO forum.poll_options (id, poll_id, text, position) \
                         VALUES ($1, $2, $3, $4)",
                        params![new_id(), poll_id, opt, i as i32],
                    )
                    .await?;
                }
            }
        }

        tx.commit().await?;

        let topic = Self::get(topic_id, db).await?;
        let post = db
            .fetch_one_as::<Post>("SELECT * FROM forum.posts WHERE id = $1", params![post_id])
            .await?;
        Ok((topic, post))
    }

    pub async fn update(id: Uuid, user: &ForumUser, dto: UpdateTopicDto, db: &DbPool) -> Result<Topic> {
        let topic = Self::get(id, db).await?;
        let perms = PermissionService::effective(topic.forum_id, user, db).await?;
        let is_mod = perms.is_admin || perms.is_moderator;
        if topic.author_id != user.id && !is_mod {
            return Err(ForumError::Forbidden);
        }
        if (dto.topic_type.is_some() || dto.is_locked.is_some()) && !is_mod {
            return Err(ForumError::Forbidden);
        }
        if let Some(t) = &dto.topic_type {
            if !VALID_TYPES.contains(&t.as_str()) {
                return Err(ForumError::Validation(format!("invalid topic type: {t}")));
            }
        }
        // When the title changes, its normalized search column follows it.
        let title_norm = dto.title.as_deref().map(normalize);
        db.execute(
            "UPDATE forum.topics SET \
                title      = COALESCE($1, title), \
                topic_type = COALESCE($2, topic_type), \
                is_locked  = COALESCE($3, is_locked), \
                title_norm = COALESCE($4, title_norm) \
             WHERE id = $5",
            params![dto.title, dto.topic_type, dto.is_locked, title_norm, id],
        )
        .await?;
        Self::get(id, db).await
    }

    pub async fn delete(id: Uuid, user: &ForumUser, db: &DbPool) -> Result<()> {
        let topic = Self::get(id, db).await?;
        let perms = PermissionService::effective(topic.forum_id, user, db).await?;
        if topic.author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }
        let mut tx = db.begin().await?;
        // Soft-delete rather than erase (SEC-12).
        let affected = tx
            .execute(
                "UPDATE forum.topics SET is_deleted = TRUE, deleted_at = $1, deleted_by = $2 \
                  WHERE id = $3 AND is_deleted = FALSE",
                params![Utc::now(), user.id, id],
            )
            .await?;
        if affected == 0 {
            tx.rollback().await?;
            return Err(ForumError::NotFound(format!("Topic {id}")));
        }
        aggregates::recompute_forum(&mut tx, topic.forum_id).await?;
        tx.commit().await?;
        Ok(())
    }

    // ── Trash (soft-deleted topics) ─────────────────────────────────────────────

    /// Soft-deleted topics, newest deletion first.
    pub async fn list_deleted(forum_ids: Option<&[Uuid]>, limit: i64, db: &DbPool) -> Result<Vec<Topic>> {
        let limit = limit.clamp(1, 200);
        match forum_ids {
            None => Ok(db
                .fetch_all_as::<Topic>(
                    "SELECT * FROM forum.topics WHERE is_deleted = TRUE \
                     ORDER BY deleted_at DESC LIMIT $1",
                    params![limit],
                )
                .await?),
            Some(ids) => {
                let mut qb = DbQueryBuilder::new(
                    db.backend(),
                    "SELECT * FROM forum.topics WHERE is_deleted = TRUE AND forum_id",
                );
                qb.push_in(ids.iter().copied())
                    .push(" ORDER BY deleted_at DESC LIMIT ")
                    .push_bind(limit);
                Ok(qb.fetch_all_as::<Topic>(db).await?)
            }
        }
    }

    /// Brings a soft-deleted topic back into every listing.
    pub async fn restore(id: Uuid, db: &DbPool) -> Result<Topic> {
        let topic = Self::get(id, db).await?;
        let mut tx = db.begin().await?;
        let affected = tx
            .execute(
                "UPDATE forum.topics SET is_deleted = FALSE, deleted_at = NULL, deleted_by = NULL \
                  WHERE id = $1 AND is_deleted = TRUE",
                params![id],
            )
            .await?;
        if affected == 0 {
            tx.rollback().await?;
            return Err(ForumError::NotFound(format!("Topic {id}")));
        }
        aggregates::recompute_forum(&mut tx, topic.forum_id).await?;
        tx.commit().await?;
        Self::get(id, db).await
    }

    /// Permanently erases a soft-deleted topic and every post it holds.
    pub async fn purge(id: Uuid, db: &DbPool) -> Result<()> {
        let mut tx = db.begin().await?;
        let forum_id: Option<Uuid> = tx
            .fetch_optional_scalar(
                "SELECT forum_id FROM forum.topics WHERE id = $1 AND is_deleted = TRUE",
                params![id],
            )
            .await?;
        let Some(forum_id) = forum_id else {
            tx.rollback().await?;
            return Err(ForumError::NotFound(format!("Topic {id}")));
        };
        tx.execute(
            "DELETE FROM forum.topics WHERE id = $1 AND is_deleted = TRUE",
            params![id],
        )
        .await?;
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        tx.commit().await?;
        Ok(())
    }

    // ── Moderation actions ────────────────────────────────────────────────────

    pub async fn set_locked(id: Uuid, locked: bool, db: &DbPool) -> Result<Topic> {
        let affected = db
            .execute(
                "UPDATE forum.topics SET is_locked = $1 WHERE id = $2",
                params![locked, id],
            )
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Topic {id}")));
        }
        Self::get(id, db).await
    }

    /// Marks a post as the accepted answer.
    pub async fn set_solution(id: Uuid, post_id: Uuid, user: &ForumUser, db: &DbPool) -> Result<(Topic, Uuid)> {
        let topic = Self::get(id, db).await?;
        let perms = PermissionService::effective(topic.forum_id, user, db).await?;
        if topic.author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }
        let post_author: Option<Uuid> = db
            .fetch_optional_scalar(
                "SELECT author_id FROM forum.posts WHERE id = $1 AND topic_id = $2",
                params![post_id, id],
            )
            .await?;
        let post_author =
            post_author.ok_or_else(|| ForumError::Validation("post is not in this topic".into()))?;

        db.execute(
            "UPDATE forum.topics SET is_solved = TRUE, solution_post_id = $1 WHERE id = $2",
            params![post_id, id],
        )
        .await?;
        let topic = Self::get(id, db).await?;
        Ok((topic, post_author))
    }

    pub async fn clear_solution(id: Uuid, user: &ForumUser, db: &DbPool) -> Result<Topic> {
        let topic = Self::get(id, db).await?;
        let perms = PermissionService::effective(topic.forum_id, user, db).await?;
        if topic.author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }
        db.execute(
            "UPDATE forum.topics SET is_solved = FALSE, solution_post_id = NULL WHERE id = $1",
            params![id],
        )
        .await?;
        Self::get(id, db).await
    }

    pub async fn move_to(id: Uuid, dto: MoveTopicDto, db: &DbPool) -> Result<Topic> {
        let topic = Self::get(id, db).await?;
        let old_forum = topic.forum_id;
        let mut tx = db.begin().await?;
        tx.execute(
            "UPDATE forum.topics SET forum_id = $1 WHERE id = $2",
            params![dto.forum_id, id],
        )
        .await?;
        tx.execute(
            "UPDATE forum.posts SET forum_id = $1 WHERE topic_id = $2",
            params![dto.forum_id, id],
        )
        .await?;
        aggregates::recompute_forum(&mut tx, old_forum).await?;
        aggregates::recompute_forum(&mut tx, dto.forum_id).await?;
        tx.commit().await?;
        Self::get(id, db).await
    }

    /// Move a set of posts into a brand-new topic.
    pub async fn split(id: Uuid, author_id: Uuid, dto: SplitTopicDto, db: &DbPool) -> Result<Topic> {
        let source = Self::get(id, db).await?;
        if dto.post_ids.is_empty() {
            return Err(ForumError::Validation("no posts selected".into()));
        }
        let target_forum = dto.forum_id.unwrap_or(source.forum_id);
        let slug = aggregates::slugify(&dto.title);
        let title_norm = normalize(&dto.title);
        let new_topic_id = new_id();

        let mut tx = db.begin().await?;
        tx.execute(
            "INSERT INTO forum.topics (id, forum_id, author_id, title, slug, topic_type, title_norm) \
             VALUES ($1, $2, $3, $4, $5, 'normal', $6)",
            params![
                new_topic_id,
                target_forum,
                author_id,
                dto.title,
                slug,
                title_norm
            ],
        )
        .await?;

        // Reassign the selected posts (only those actually in the source topic).
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "UPDATE forum.posts SET topic_id = ",
        );
        qb.push_bind(new_topic_id)
            .push(", forum_id = ")
            .push_bind(target_forum)
            .push(", is_first_post = FALSE WHERE id")
            .push_in(dto.post_ids.iter().copied())
            .push(" AND topic_id = ")
            .push_bind(id);
        qb.tx_execute(&mut tx).await?;

        // Promote the earliest moved post to the new topic's first post.
        tx.execute(
            "UPDATE forum.posts SET is_first_post = TRUE \
             WHERE id = (SELECT id FROM forum.posts WHERE topic_id = $1 ORDER BY created_at, id LIMIT 1)",
            params![new_topic_id],
        )
        .await?;

        aggregates::recompute_topic(&mut tx, id).await?;
        aggregates::recompute_topic(&mut tx, new_topic_id).await?;
        aggregates::recompute_forum(&mut tx, source.forum_id).await?;
        if target_forum != source.forum_id {
            aggregates::recompute_forum(&mut tx, target_forum).await?;
        }
        tx.commit().await?;
        Self::get(new_topic_id, db).await
    }

    /// Merge another topic's posts into this one, then delete the source topic.
    pub async fn merge(id: Uuid, dto: MergeTopicDto, db: &DbPool) -> Result<Topic> {
        if id == dto.source_topic_id {
            return Err(ForumError::Validation("cannot merge a topic into itself".into()));
        }
        let target = Self::get(id, db).await?;
        let source = Self::get(dto.source_topic_id, db).await?;

        let mut tx = db.begin().await?;
        tx.execute(
            "UPDATE forum.posts SET topic_id = $1, forum_id = $2, is_first_post = FALSE \
             WHERE topic_id = $3",
            params![id, target.forum_id, source.id],
        )
        .await?;
        // Keep exactly one first post (the earliest of the merged set).
        tx.execute(
            "UPDATE forum.posts SET is_first_post = FALSE WHERE topic_id = $1",
            params![id],
        )
        .await?;
        tx.execute(
            "UPDATE forum.posts SET is_first_post = TRUE \
             WHERE id = (SELECT id FROM forum.posts WHERE topic_id = $1 ORDER BY created_at, id LIMIT 1)",
            params![id],
        )
        .await?;
        tx.execute("DELETE FROM forum.topics WHERE id = $1", params![source.id])
            .await?;

        aggregates::recompute_topic(&mut tx, id).await?;
        aggregates::recompute_forum(&mut tx, target.forum_id).await?;
        if source.forum_id != target.forum_id {
            aggregates::recompute_forum(&mut tx, source.forum_id).await?;
        }
        tx.commit().await?;
        Self::get(id, db).await
    }

    // ── Maintenance ──────────────────────────────────────────────────────────

    /// Counts, or soft-deletes, the topics of `forum_id` inactive for at least
    /// `older_than_days`. `dry_run = true` only counts.
    pub async fn prune_forum(
        forum_id: Uuid,
        older_than_days: i32,
        dry_run: bool,
        actor: Uuid,
        db: &DbPool,
    ) -> Result<i64> {
        if older_than_days < 1 {
            return Err(ForumError::Validation("older_than_days must be at least 1".into()));
        }
        let cutoff = Utc::now() - Duration::days(older_than_days as i64);

        if dry_run {
            let count: i64 = db
                .fetch_scalar(
                    concat!(
                        "SELECT COUNT(*) FROM forum.topics WHERE",
                        prune_predicate!("$1", "$2")
                    ),
                    params![forum_id, cutoff],
                )
                .await?;
            return Ok(count);
        }

        let mut tx = db.begin().await?;
        // The UPDATE's own placeholders come first ($1, $2), then the predicate's
        // ($3, $4), so the numbering stays strictly increasing left to right.
        let affected = tx
            .execute(
                concat!(
                    "UPDATE forum.topics SET is_deleted = TRUE, deleted_at = $1, deleted_by = $2 WHERE",
                    prune_predicate!("$3", "$4")
                ),
                params![Utc::now(), actor, forum_id, cutoff],
            )
            .await?;
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        tx.commit().await?;
        Ok(affected as i64)
    }
}
