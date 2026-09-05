use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
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
        db: &PgPool,
    ) -> Result<Vec<Topic>> {
        // Pinned types first (global, announcement, sticky), then by latest activity.
        let rows = sqlx::query_as::<_, Topic>(
            "SELECT * FROM forum.topics
              WHERE forum_id = $1 AND is_deleted = FALSE
                AND (is_approved = TRUE OR author_id = $2 OR $3::boolean)
             ORDER BY CASE topic_type
                        WHEN 'global'       THEN 0
                        WHEN 'announcement' THEN 1
                        WHEN 'sticky'       THEN 2
                        ELSE 3 END,
                      last_post_at DESC NULLS LAST, created_at DESC
             LIMIT $4 OFFSET $5",
        )
        .bind(forum_id)
        .bind(viewer_id)
        .bind(is_moderator)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn count_by_forum(
        forum_id: Uuid,
        viewer_id: Uuid,
        is_moderator: bool,
        db: &PgPool,
    ) -> Result<i64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM forum.topics
              WHERE forum_id = $1 AND is_deleted = FALSE
                AND (is_approved = TRUE OR author_id = $2 OR $3::boolean)",
        )
        .bind(forum_id)
        .bind(viewer_id)
        .bind(is_moderator)
        .fetch_one(db)
        .await?;
        Ok(n)
    }

    pub async fn get(id: Uuid, db: &PgPool) -> Result<Topic> {
        sqlx::query_as::<_, Topic>("SELECT * FROM forum.topics WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("Topic {id}")))
    }

    /// This user's read marker for one topic — `None` when no row exists yet
    /// (the topic has never been marked read by them). Queried directly
    /// against `read_markers` rather than through `EngagementService`, which
    /// this read-only lookup (the topic view's "jump to first unread post")
    /// has no other reason to depend on.
    pub async fn read_marker(topic_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Option<DateTime<Utc>>> {
        let read_at: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT read_at FROM forum.read_markers WHERE topic_id = $1 AND user_id = $2",
        )
        .bind(topic_id)
        .bind(user_id)
        .fetch_optional(db)
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
        db: &PgPool,
    ) -> Result<Vec<Topic>> {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT t.* FROM forum.topics t ");
        if kind == "unread" {
            qb.push("LEFT JOIN forum.read_markers rm ON rm.topic_id = t.id AND rm.user_id = ");
            qb.push_bind(user_id);
            qb.push(" ");
        }
        qb.push("WHERE NOT EXISTS (SELECT 1 FROM forum.permissions pm \
                 WHERE pm.forum_id = t.forum_id AND pm.role = 'user' AND pm.can_view = FALSE) ");
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

        let order = match kind {
            "popular" => "ORDER BY (t.view_count + t.reply_count * 3) DESC, t.last_post_at DESC NULLS LAST ",
            "unanswered" | "mine" => "ORDER BY t.created_at DESC ",
            _ => "ORDER BY t.last_post_at DESC NULLS LAST, t.created_at DESC ",
        };
        qb.push(order);
        qb.push("LIMIT ").push_bind(limit.clamp(1, 100)).push(" OFFSET ").push_bind(offset.max(0));

        let rows = qb.build_query_as::<Topic>().fetch_all(db).await?;
        Ok(rows)
    }

    /// Total topics matching `feed`'s filters — same WHERE clause, no
    /// ORDER/LIMIT, so pagination on the discovery feed never counts (or
    /// implies) a topic the caller would not otherwise be shown.
    pub async fn feed_count(
        kind: &str,
        user_id: Uuid,
        solved: Option<bool>,
        tag_id: Option<Uuid>,
        db: &PgPool,
    ) -> Result<i64> {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM forum.topics t ");
        if kind == "unread" {
            qb.push("LEFT JOIN forum.read_markers rm ON rm.topic_id = t.id AND rm.user_id = ");
            qb.push_bind(user_id);
            qb.push(" ");
        }
        qb.push("WHERE NOT EXISTS (SELECT 1 FROM forum.permissions pm \
                 WHERE pm.forum_id = t.forum_id AND pm.role = 'user' AND pm.can_view = FALSE) ");
        qb.push("AND t.is_deleted = FALSE ");
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

        let n: i64 = qb.build_query_scalar().fetch_one(db).await?;
        Ok(n)
    }

    /// Topics authored by a given user (public profile listing).
    pub async fn by_author(author_id: Uuid, limit: i64, db: &PgPool) -> Result<Vec<Topic>> {
        // Public listing: approved only. A member's profile is read by everyone,
        // so it must not be the place where a queued topic becomes visible.
        let rows = sqlx::query_as::<_, Topic>(
            "SELECT * FROM forum.topics WHERE author_id = $1 AND is_approved \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(author_id)
        .bind(limit.clamp(1, 100))
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn touch_view(id: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("UPDATE forum.topics SET view_count = view_count + 1 WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }

    /// Create a topic together with its opening post. `topic_type` must already be
    /// authorised by the caller (only moderators/admins may pin topics).
    ///
    /// `approved` decides whether the topic appears at once or waits in the
    /// moderation queue. A topic and its opening message share one fate: a topic
    /// whose only message is hidden would be an empty shell in the listing. The
    /// author's counters are only bumped on publication — see `PostService::create`.
    pub async fn create(
        forum_id: Uuid,
        author_id: Uuid,
        topic_type: &str,
        dto: CreateTopicDto,
        approved: bool,
        db: &PgPool,
    ) -> Result<(Topic, Post)> {
        if !VALID_TYPES.contains(&topic_type) {
            return Err(ForumError::Validation(format!("invalid topic type: {topic_type}")));
        }
        let slug = aggregates::slugify(&dto.title);

        let mut tx = db.begin().await?;
        let topic = sqlx::query_as::<_, Topic>(
            "INSERT INTO forum.topics
                (forum_id, author_id, title, slug, topic_type, is_question, prefix,
                 is_approved, approved_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CASE WHEN $8::boolean THEN NOW() END) RETURNING *",
        )
        .bind(forum_id)
        .bind(author_id)
        .bind(&dto.title)
        .bind(&slug)
        .bind(topic_type)
        .bind(dto.is_question)
        .bind(dto.prefix.as_deref().filter(|s| !s.is_empty()))
        .bind(approved)
        .fetch_one(&mut *tx)
        .await?;

        let post = sqlx::query_as::<_, Post>(
            "INSERT INTO forum.posts
                (topic_id, forum_id, author_id, body_md, is_first_post, is_approved, approved_at)
             VALUES ($1, $2, $3, $4, TRUE, $5, CASE WHEN $5::boolean THEN NOW() END) RETURNING *",
        )
        .bind(topic.id)
        .bind(forum_id)
        .bind(author_id)
        .bind(&dto.body_md)
        .bind(approved)
        .fetch_one(&mut *tx)
        .await?;

        aggregates::recompute_topic(&mut tx, topic.id).await?;
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        if approved {
            RankService::bump_post_count(&mut tx, author_id, 1).await?;
            RankService::bump_topic_count(&mut tx, author_id, 1).await?;
        }

        // Attach selected tags (only ones that exist).
        for tag_id in &dto.tag_ids {
            sqlx::query(
                "INSERT INTO forum.topic_tags (topic_id, tag_id)
                 SELECT $1, $2 WHERE EXISTS (SELECT 1 FROM forum.tags WHERE id = $2)
                 ON CONFLICT DO NOTHING",
            )
            .bind(topic.id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
        }

        // Attach an optional poll.
        if let Some(poll) = &dto.poll {
            let opts: Vec<String> = poll.options.iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            if opts.len() >= 2 {
                let poll_id: Uuid = sqlx::query_scalar(
                    "INSERT INTO forum.polls (topic_id, question, is_multiple, closes_at)
                     VALUES ($1, $2, $3, $4) RETURNING id",
                )
                .bind(topic.id)
                .bind(&poll.question)
                .bind(poll.is_multiple)
                .bind(poll.closes_at)
                .fetch_one(&mut *tx)
                .await?;
                for (i, opt) in opts.iter().enumerate() {
                    sqlx::query("INSERT INTO forum.poll_options (poll_id, text, position) VALUES ($1, $2, $3)")
                        .bind(poll_id)
                        .bind(opt)
                        .bind(i as i32)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }

        tx.commit().await?;

        let topic = Self::get(topic.id, db).await?;
        Ok((topic, post))
    }

    pub async fn update(id: Uuid, user: &ForumUser, dto: UpdateTopicDto, db: &PgPool) -> Result<Topic> {
        let topic = Self::get(id, db).await?;
        let perms = PermissionService::effective(topic.forum_id, user, db).await?;
        let is_mod = perms.is_admin || perms.is_moderator;
        // The author may rename their own topic; only moderators change type/lock.
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
        let row = sqlx::query_as::<_, Topic>(
            "UPDATE forum.topics SET
                title      = COALESCE($2, title),
                topic_type = COALESCE($3, topic_type),
                is_locked  = COALESCE($4, is_locked)
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(&dto.title)
        .bind(&dto.topic_type)
        .bind(dto.is_locked)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn delete(id: Uuid, user: &ForumUser, db: &PgPool) -> Result<()> {
        let topic = Self::get(id, db).await?;
        let perms = PermissionService::effective(topic.forum_id, user, db).await?;
        if topic.author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }
        let mut tx = db.begin().await?;
        // Soft-delete rather than erase (SEC-12): the topic drops out of every
        // listing but its posts stay in place and it can be recovered. A wrong
        // click, or a compromised account, no longer destroys a discussion for
        // good. Recomputing the forum afterwards discounts the now-hidden topic.
        let r = sqlx::query(
            "UPDATE forum.topics SET is_deleted = TRUE, deleted_at = NOW(), deleted_by = $2
              WHERE id = $1 AND is_deleted = FALSE",
        )
        .bind(id)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(ForumError::NotFound(format!("Topic {id}")));
        }
        aggregates::recompute_forum(&mut tx, topic.forum_id).await?;
        tx.commit().await?;
        Ok(())
    }

    // ── Trash (soft-deleted topics) ─────────────────────────────────────────────

    /// Soft-deleted topics, newest deletion first. `forum_ids = None` is every
    /// forum (platform administrators); otherwise only the forums the caller
    /// moderates, mirroring the scoping already used for reports/queue/log.
    pub async fn list_deleted(forum_ids: Option<&[Uuid]>, limit: i64, db: &PgPool) -> Result<Vec<Topic>> {
        let rows = match forum_ids {
            None => sqlx::query_as::<_, Topic>(
                "SELECT * FROM forum.topics WHERE is_deleted = TRUE
                 ORDER BY deleted_at DESC LIMIT $1",
            )
            .bind(limit.clamp(1, 200))
            .fetch_all(db)
            .await?,
            Some(ids) => sqlx::query_as::<_, Topic>(
                "SELECT * FROM forum.topics WHERE is_deleted = TRUE AND forum_id = ANY($1)
                 ORDER BY deleted_at DESC LIMIT $2",
            )
            .bind(ids)
            .bind(limit.clamp(1, 200))
            .fetch_all(db)
            .await?,
        };
        Ok(rows)
    }

    /// Brings a soft-deleted topic back into every listing. A no-op target (not
    /// deleted, or gone) is reported as not-found rather than silently accepted.
    pub async fn restore(id: Uuid, db: &PgPool) -> Result<Topic> {
        let mut tx = db.begin().await?;
        let r = sqlx::query(
            "UPDATE forum.topics SET is_deleted = FALSE, deleted_at = NULL, deleted_by = NULL
              WHERE id = $1 AND is_deleted = TRUE",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(ForumError::NotFound(format!("Topic {id}")));
        }
        let topic = sqlx::query_as::<_, Topic>("SELECT * FROM forum.topics WHERE id = $1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        aggregates::recompute_forum(&mut tx, topic.forum_id).await?;
        tx.commit().await?;
        Ok(topic)
    }

    /// Permanently erases a soft-deleted topic and every post it holds. This is
    /// the only hard delete in the topic lifecycle (SEC-12): reserved for admins,
    /// and only ever applied to something already sitting in the trash, so a
    /// moderator's soft-delete remains the sole path to losing a topic's content.
    pub async fn purge(id: Uuid, db: &PgPool) -> Result<()> {
        let mut tx = db.begin().await?;
        let forum_id: Option<Uuid> = sqlx::query_scalar(
            "DELETE FROM forum.topics WHERE id = $1 AND is_deleted = TRUE RETURNING forum_id",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(forum_id) = forum_id else {
            tx.rollback().await?;
            return Err(ForumError::NotFound(format!("Topic {id}")));
        };
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        tx.commit().await?;
        Ok(())
    }

    // ── Moderation actions ────────────────────────────────────────────────────

    pub async fn set_locked(id: Uuid, locked: bool, db: &PgPool) -> Result<Topic> {
        sqlx::query_as::<_, Topic>("UPDATE forum.topics SET is_locked = $2 WHERE id = $1 RETURNING *")
            .bind(id)
            .bind(locked)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("Topic {id}")))
    }

    /// Marks a post as the accepted answer. Only the topic author or a
    /// moderator/admin may do so. Returns the updated topic and the solution
    /// post's author (for notification).
    pub async fn set_solution(id: Uuid, post_id: Uuid, user: &ForumUser, db: &PgPool) -> Result<(Topic, Uuid)> {
        let topic = Self::get(id, db).await?;
        let perms = PermissionService::effective(topic.forum_id, user, db).await?;
        if topic.author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }
        let post_author: Option<Uuid> = sqlx::query_scalar(
            "SELECT author_id FROM forum.posts WHERE id = $1 AND topic_id = $2",
        )
        .bind(post_id)
        .bind(id)
        .fetch_optional(db)
        .await?;
        let post_author = post_author.ok_or_else(|| ForumError::Validation("post is not in this topic".into()))?;

        let topic = sqlx::query_as::<_, Topic>(
            "UPDATE forum.topics SET is_solved = TRUE, solution_post_id = $2 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(post_id)
        .fetch_one(db)
        .await?;
        Ok((topic, post_author))
    }

    pub async fn clear_solution(id: Uuid, user: &ForumUser, db: &PgPool) -> Result<Topic> {
        let topic = Self::get(id, db).await?;
        let perms = PermissionService::effective(topic.forum_id, user, db).await?;
        if topic.author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }
        sqlx::query_as::<_, Topic>(
            "UPDATE forum.topics SET is_solved = FALSE, solution_post_id = NULL WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(db)
        .await
        .map_err(ForumError::Database)
    }

    pub async fn move_to(id: Uuid, dto: MoveTopicDto, db: &PgPool) -> Result<Topic> {
        let topic = Self::get(id, db).await?;
        let old_forum = topic.forum_id;
        let mut tx = db.begin().await?;
        sqlx::query("UPDATE forum.topics SET forum_id = $2 WHERE id = $1")
            .bind(id)
            .bind(dto.forum_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE forum.posts SET forum_id = $2 WHERE topic_id = $1")
            .bind(id)
            .bind(dto.forum_id)
            .execute(&mut *tx)
            .await?;
        aggregates::recompute_forum(&mut tx, old_forum).await?;
        aggregates::recompute_forum(&mut tx, dto.forum_id).await?;
        tx.commit().await?;
        Self::get(id, db).await
    }

    /// Move a set of posts into a brand-new topic.
    pub async fn split(id: Uuid, author_id: Uuid, dto: SplitTopicDto, db: &PgPool) -> Result<Topic> {
        let source = Self::get(id, db).await?;
        if dto.post_ids.is_empty() {
            return Err(ForumError::Validation("no posts selected".into()));
        }
        let target_forum = dto.forum_id.unwrap_or(source.forum_id);
        let slug = aggregates::slugify(&dto.title);

        let mut tx = db.begin().await?;
        let new_topic = sqlx::query_as::<_, Topic>(
            "INSERT INTO forum.topics (forum_id, author_id, title, slug, topic_type)
             VALUES ($1, $2, $3, $4, 'normal') RETURNING *",
        )
        .bind(target_forum)
        .bind(author_id)
        .bind(&dto.title)
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await?;

        // Reassign the selected posts (only those actually in the source topic).
        sqlx::query(
            "UPDATE forum.posts SET topic_id = $1, forum_id = $2, is_first_post = FALSE
             WHERE id = ANY($3) AND topic_id = $4",
        )
        .bind(new_topic.id)
        .bind(target_forum)
        .bind(&dto.post_ids)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Promote the earliest moved post to the new topic's first post.
        sqlx::query(
            "UPDATE forum.posts SET is_first_post = TRUE
             WHERE id = (SELECT id FROM forum.posts WHERE topic_id = $1 ORDER BY created_at, id LIMIT 1)",
        )
        .bind(new_topic.id)
        .execute(&mut *tx)
        .await?;

        aggregates::recompute_topic(&mut tx, id).await?;
        aggregates::recompute_topic(&mut tx, new_topic.id).await?;
        aggregates::recompute_forum(&mut tx, source.forum_id).await?;
        if target_forum != source.forum_id {
            aggregates::recompute_forum(&mut tx, target_forum).await?;
        }
        tx.commit().await?;
        Self::get(new_topic.id, db).await
    }

    /// Merge another topic's posts into this one, then delete the source topic.
    pub async fn merge(id: Uuid, dto: MergeTopicDto, db: &PgPool) -> Result<Topic> {
        if id == dto.source_topic_id {
            return Err(ForumError::Validation("cannot merge a topic into itself".into()));
        }
        let target = Self::get(id, db).await?;
        let source = Self::get(dto.source_topic_id, db).await?;

        let mut tx = db.begin().await?;
        sqlx::query(
            "UPDATE forum.posts SET topic_id = $1, forum_id = $2, is_first_post = FALSE
             WHERE topic_id = $3",
        )
        .bind(id)
        .bind(target.forum_id)
        .bind(source.id)
        .execute(&mut *tx)
        .await?;
        // Keep exactly one first post (the earliest of the merged set).
        sqlx::query("UPDATE forum.posts SET is_first_post = FALSE WHERE topic_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE forum.posts SET is_first_post = TRUE
             WHERE id = (SELECT id FROM forum.posts WHERE topic_id = $1 ORDER BY created_at, id LIMIT 1)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM forum.topics WHERE id = $1")
            .bind(source.id)
            .execute(&mut *tx)
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

    /// WHERE clause shared by `prune_forum`'s preview and its actual pass: stale,
    /// non-deleted topics of one forum. Pinned types, a solved question, and any
    /// topic carrying a poll are excluded — a prune is meant for dead ordinary
    /// discussions, not content an admin (or a poll's voters) expects to keep
    /// finding in place. `$1` = forum_id, `$2` = older_than_days.
    const PRUNE_PREDICATE: &'static str = "
        forum_id = $1 AND is_deleted = FALSE
        AND COALESCE(last_post_at, created_at) < NOW() - make_interval(days => $2)
        AND topic_type NOT IN ('sticky', 'announcement', 'global')
        AND is_solved = FALSE
        AND NOT EXISTS (SELECT 1 FROM forum.polls WHERE topic_id = forum.topics.id)";

    /// Counts, or soft-deletes, the topics of `forum_id` that have sat inactive
    /// for at least `older_than_days`. `dry_run = true` only counts — nothing is
    /// written — so an admin can preview the impact before committing to it.
    /// Otherwise the matching topics are soft-deleted in one transaction (same
    /// mechanism as `delete`, fully reversible from the trash) and the forum's
    /// aggregates are recomputed to drop them from its counters.
    pub async fn prune_forum(
        forum_id: Uuid,
        older_than_days: i32,
        dry_run: bool,
        actor: Uuid,
        db: &PgPool,
    ) -> Result<i64> {
        if older_than_days < 1 {
            return Err(ForumError::Validation("older_than_days must be at least 1".into()));
        }

        if dry_run {
            let count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM forum.topics WHERE {}",
                Self::PRUNE_PREDICATE
            ))
            .bind(forum_id)
            .bind(older_than_days)
            .fetch_one(db)
            .await?;
            return Ok(count);
        }

        let mut tx = db.begin().await?;
        let pruned: Vec<Uuid> = sqlx::query_scalar(&format!(
            "UPDATE forum.topics SET is_deleted = TRUE, deleted_at = NOW(), deleted_by = $3
              WHERE {} RETURNING id",
            Self::PRUNE_PREDICATE
        ))
        .bind(forum_id)
        .bind(older_than_days)
        .bind(actor)
        .fetch_all(&mut *tx)
        .await?;
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        tx.commit().await?;
        Ok(pruned.len() as i64)
    }
}
