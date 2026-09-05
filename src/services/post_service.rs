use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config::instance::InstanceConfig,
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::post::{CreatePostDto, Post, PostRevision, UpdatePostDto},
    services::{aggregates, permission_service::PermissionService, rank_service::RankService},
};

pub struct PostService;

impl PostService {
    /// Messages of a topic, as this viewer may see them.
    ///
    /// A message waiting for approval is shown to its own author — otherwise
    /// people re-post, thinking the forum swallowed what they wrote — and to
    /// moderators, who have to be able to read what they are deciding on.
    /// Everyone else sees the topic without it.
    pub async fn list_by_topic(
        topic_id: Uuid,
        viewer_id: Uuid,
        is_moderator: bool,
        limit: i64,
        offset: i64,
        db: &PgPool,
    ) -> Result<Vec<Post>> {
        let rows = sqlx::query_as::<_, Post>(
            "SELECT * FROM forum.posts
              WHERE topic_id = $1 AND is_deleted = FALSE
                AND (is_approved = TRUE OR author_id = $2 OR $3::boolean)
             ORDER BY created_at, id LIMIT $4 OFFSET $5",
        )
        .bind(topic_id)
        .bind(viewer_id)
        .bind(is_moderator)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn count_by_topic(
        topic_id: Uuid,
        viewer_id: Uuid,
        is_moderator: bool,
        db: &PgPool,
    ) -> Result<i64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM forum.posts
              WHERE topic_id = $1 AND is_deleted = FALSE
                AND (is_approved = TRUE OR author_id = $2 OR $3::boolean)",
        )
        .bind(topic_id)
        .bind(viewer_id)
        .bind(is_moderator)
        .fetch_one(db)
        .await?;
        Ok(n)
    }

    pub async fn get(id: Uuid, db: &PgPool) -> Result<Post> {
        sqlx::query_as::<_, Post>("SELECT * FROM forum.posts WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("Post {id}")))
    }

    /// Append a reply to a topic. Maintains counters and the author's post count.
    ///
    /// `approved` decides whether the message is published straight away or held
    /// in the moderation queue. A held message deliberately does NOT bump the
    /// author's post count: that count is what the "new member" rule reads, so
    /// counting unapproved messages would let anyone queue their way out of
    /// being moderated. `ModerationService::approve_post` bumps it on release.
    pub async fn create(
        topic_id: Uuid,
        forum_id: Uuid,
        author_id: Uuid,
        dto: CreatePostDto,
        approved: bool,
        db: &PgPool,
    ) -> Result<Post> {
        let mut tx = db.begin().await?;

        let post = sqlx::query_as::<_, Post>(
            "INSERT INTO forum.posts
                (topic_id, forum_id, author_id, body_md, reply_to_post_id, is_first_post,
                 is_approved, approved_at)
             VALUES ($1, $2, $3, $4, $5, FALSE, $6, CASE WHEN $6::boolean THEN NOW() END) RETURNING *",
        )
        .bind(topic_id)
        .bind(forum_id)
        .bind(author_id)
        .bind(&dto.body_md)
        .bind(dto.reply_to_post_id)
        .bind(approved)
        .fetch_one(&mut *tx)
        .await?;

        aggregates::recompute_topic(&mut tx, topic_id).await?;
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        if approved {
            RankService::bump_post_count(&mut tx, author_id, 1).await?;
        }

        tx.commit().await?;
        Ok(post)
    }

    /// Timestamp of this author's most recent message, for flood control.
    pub async fn last_post_at(
        author_id: Uuid,
        db: &PgPool,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        let ts: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "SELECT MAX(created_at) FROM forum.posts WHERE author_id = $1",
        )
        .bind(author_id)
        .fetch_one(db)
        .await?;
        Ok(ts)
    }

    /// Refuses a contribution that comes too soon after the author's previous
    /// one. Moderators are exempt: flood control exists against drive-by
    /// spamming, and holding up the people who clean it up helps nobody.
    pub async fn assert_not_flooding(
        author_id: Uuid,
        is_moderator: bool,
        cfg: &InstanceConfig,
        db: &PgPool,
    ) -> Result<()> {
        if is_moderator || cfg.min_seconds_between_posts <= 0 {
            return Ok(());
        }
        let Some(last) = Self::last_post_at(author_id, db).await? else {
            return Ok(());
        };
        let elapsed = (chrono::Utc::now() - last).num_seconds();
        if elapsed < cfg.min_seconds_between_posts {
            return Err(ForumError::RateLimited(cfg.min_seconds_between_posts - elapsed));
        }
        Ok(())
    }

    pub async fn update(
        id: Uuid,
        user: &ForumUser,
        dto: UpdatePostDto,
        cfg: &InstanceConfig,
        db: &PgPool,
    ) -> Result<Post> {
        let post = Self::get(id, db).await?;
        // A message removed by moderation is not editable back into existence.
        if post.is_deleted {
            return Err(ForumError::NotFound(format!("Post {id}")));
        }
        let perms = PermissionService::effective(post.forum_id, user, db).await?;
        let is_mod = perms.is_admin || perms.is_moderator;
        if post.author_id != user.id && !is_mod {
            return Err(ForumError::Forbidden);
        }

        // Edit window: an author may correct themselves for a while, not rewrite
        // history a month later under a reply that answered the old text.
        // Moderators are never bound by it — that is what makes them moderators.
        if !is_mod && cfg.post_edit_window_minutes > 0 {
            let age = (chrono::Utc::now() - post.created_at).num_minutes();
            if age > cfg.post_edit_window_minutes {
                return Err(ForumError::Forbidden);
            }
        }

        // Archive the version being overwritten (its body plus whoever produced
        // it) before touching the row, in the same transaction as the update so
        // an edit is never recorded without its predecessor being preserved.
        let mut tx = db.begin().await?;

        sqlx::query(
            "INSERT INTO forum.post_revisions (post_id, body_md, edited_by, edit_reason)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(&post.body_md)
        .bind(post.edited_by)
        .bind(&post.edit_reason)
        .execute(&mut *tx)
        .await?;

        let row = sqlx::query_as::<_, Post>(
            "UPDATE forum.posts SET
                body_md     = $2,
                edited_at   = NOW(),
                edited_by   = $3,
                edit_reason = $4,
                edit_count  = edit_count + 1
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(&dto.body_md)
        .bind(user.id)
        .bind(&dto.edit_reason)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row)
    }

    /// Edit history of a post, newest first. Access control is the caller's
    /// job (see `handlers::posts::list_revisions`).
    pub async fn list_revisions(post_id: Uuid, db: &PgPool) -> Result<Vec<PostRevision>> {
        let rows = sqlx::query_as::<_, PostRevision>(
            "SELECT * FROM forum.post_revisions WHERE post_id = $1 ORDER BY created_at DESC",
        )
        .bind(post_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn delete(id: Uuid, user: &ForumUser, db: &PgPool) -> Result<()> {
        let post = Self::get(id, db).await?;
        if post.is_first_post {
            return Err(ForumError::Conflict(
                "the first post cannot be deleted; delete the topic instead".into(),
            ));
        }
        let perms = PermissionService::effective(post.forum_id, user, db).await?;
        if post.author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }

        let mut tx = db.begin().await?;
        sqlx::query("DELETE FROM forum.posts WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        aggregates::recompute_topic(&mut tx, post.topic_id).await?;
        aggregates::recompute_forum(&mut tx, post.forum_id).await?;
        // Only an approved message was ever counted, so only that one is undone.
        if post.is_approved {
            RankService::bump_post_count(&mut tx, post.author_id, -1).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
