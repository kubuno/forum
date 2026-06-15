use sqlx::PgPool;
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
    pub async fn list_by_forum(forum_id: Uuid, limit: i64, offset: i64, db: &PgPool) -> Result<Vec<Topic>> {
        // Pinned types first (global, announcement, sticky), then by latest activity.
        let rows = sqlx::query_as::<_, Topic>(
            "SELECT * FROM forum.topics WHERE forum_id = $1
             ORDER BY CASE topic_type
                        WHEN 'global'       THEN 0
                        WHEN 'announcement' THEN 1
                        WHEN 'sticky'       THEN 2
                        ELSE 3 END,
                      last_post_at DESC NULLS LAST, created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(forum_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn count_by_forum(forum_id: Uuid, db: &PgPool) -> Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM forum.topics WHERE forum_id = $1")
            .bind(forum_id)
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

    pub async fn touch_view(id: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("UPDATE forum.topics SET view_count = view_count + 1 WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }

    /// Create a topic together with its opening post. `topic_type` must already be
    /// authorised by the caller (only moderators/admins may pin topics).
    pub async fn create(forum_id: Uuid, author_id: Uuid, topic_type: &str, dto: CreateTopicDto, db: &PgPool) -> Result<(Topic, Post)> {
        if !VALID_TYPES.contains(&topic_type) {
            return Err(ForumError::Validation(format!("invalid topic type: {topic_type}")));
        }
        let slug = aggregates::slugify(&dto.title);

        let mut tx = db.begin().await?;
        let topic = sqlx::query_as::<_, Topic>(
            "INSERT INTO forum.topics (forum_id, author_id, title, slug, topic_type)
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(forum_id)
        .bind(author_id)
        .bind(&dto.title)
        .bind(&slug)
        .bind(topic_type)
        .fetch_one(&mut *tx)
        .await?;

        let post = sqlx::query_as::<_, Post>(
            "INSERT INTO forum.posts (topic_id, forum_id, author_id, body_md, is_first_post)
             VALUES ($1, $2, $3, $4, TRUE) RETURNING *",
        )
        .bind(topic.id)
        .bind(forum_id)
        .bind(author_id)
        .bind(&dto.body_md)
        .fetch_one(&mut *tx)
        .await?;

        aggregates::recompute_topic(&mut tx, topic.id).await?;
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        RankService::bump_post_count(&mut tx, author_id, 1).await?;
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
        // Cascade removes the posts; recompute the forum afterwards.
        sqlx::query("DELETE FROM forum.topics WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        aggregates::recompute_forum(&mut tx, topic.forum_id).await?;
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
}
