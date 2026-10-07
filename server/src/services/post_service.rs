use chrono::{DateTime, Utc};
use kubuno_db::search::normalize;
use kubuno_db::{new_id, params, DbPool};
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
        db: &DbPool,
    ) -> Result<Vec<Post>> {
        let rows = db
            .fetch_all_as::<Post>(
                "SELECT * FROM forum.posts \
                  WHERE topic_id = $1 AND is_deleted = FALSE \
                    AND (is_approved = TRUE OR author_id = $2 OR $3) \
                 ORDER BY created_at, id LIMIT $4 OFFSET $5",
                params![topic_id, viewer_id, is_moderator, limit, offset],
            )
            .await?;
        Ok(rows)
    }

    pub async fn count_by_topic(
        topic_id: Uuid,
        viewer_id: Uuid,
        is_moderator: bool,
        db: &DbPool,
    ) -> Result<i64> {
        let n: i64 = db
            .fetch_scalar(
                "SELECT COUNT(*) FROM forum.posts \
                  WHERE topic_id = $1 AND is_deleted = FALSE \
                    AND (is_approved = TRUE OR author_id = $2 OR $3)",
                params![topic_id, viewer_id, is_moderator],
            )
            .await?;
        Ok(n)
    }

    pub async fn get(id: Uuid, db: &DbPool) -> Result<Post> {
        db.fetch_optional_as::<Post>("SELECT * FROM forum.posts WHERE id = $1", params![id])
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
        db: &DbPool,
    ) -> Result<Post> {
        let id = new_id();
        // approved_at is stamped only when the message is published straight away.
        let approved_at: Option<DateTime<Utc>> = if approved { Some(Utc::now()) } else { None };

        let body_norm = normalize(&dto.body_md);
        let mut tx = db.begin().await?;
        tx.execute(
            "INSERT INTO forum.posts \
                (id, topic_id, forum_id, author_id, body_md, reply_to_post_id, is_first_post, \
                 is_approved, approved_at, body_norm) \
             VALUES ($1, $2, $3, $4, $5, $6, FALSE, $7, $8, $9)",
            params![
                id,
                topic_id,
                forum_id,
                author_id,
                dto.body_md,
                dto.reply_to_post_id,
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
        }

        tx.commit().await?;
        Self::get(id, db).await
    }

    /// Timestamp of this author's most recent message, for flood control.
    pub async fn last_post_at(author_id: Uuid, db: &DbPool) -> Result<Option<DateTime<Utc>>> {
        // MAX over no rows still returns one NULL row, so decode a nullable scalar.
        let ts: Option<DateTime<Utc>> = db
            .fetch_scalar(
                "SELECT MAX(created_at) FROM forum.posts WHERE author_id = $1",
                params![author_id],
            )
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
        db: &DbPool,
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
        db: &DbPool,
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

        tx.execute(
            "INSERT INTO forum.post_revisions (id, post_id, body_md, edited_by, edit_reason) \
             VALUES ($1, $2, $3, $4, $5)",
            params![new_id(), id, post.body_md, post.edited_by, post.edit_reason],
        )
        .await?;

        let body_norm = normalize(&dto.body_md);
        tx.execute(
            "UPDATE forum.posts SET \
                body_md     = $1, \
                body_norm   = $2, \
                edited_at   = $3, \
                edited_by   = $4, \
                edit_reason = $5, \
                edit_count  = edit_count + 1 \
             WHERE id = $6",
            params![
                dto.body_md,
                body_norm,
                Utc::now(),
                user.id,
                dto.edit_reason,
                id
            ],
        )
        .await?;

        tx.commit().await?;
        Self::get(id, db).await
    }

    /// Edit history of a post, newest first. Access control is the caller's
    /// job (see `handlers::posts::list_revisions`).
    pub async fn list_revisions(post_id: Uuid, db: &DbPool) -> Result<Vec<PostRevision>> {
        let rows = db
            .fetch_all_as::<PostRevision>(
                "SELECT * FROM forum.post_revisions WHERE post_id = $1 ORDER BY created_at DESC",
                params![post_id],
            )
            .await?;
        Ok(rows)
    }

    pub async fn delete(id: Uuid, user: &ForumUser, db: &DbPool) -> Result<()> {
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
        tx.execute("DELETE FROM forum.posts WHERE id = $1", params![id]).await?;
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
