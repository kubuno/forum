use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::post::{CreatePostDto, Post, UpdatePostDto},
    services::{aggregates, permission_service::PermissionService, rank_service::RankService},
};

pub struct PostService;

impl PostService {
    pub async fn list_by_topic(topic_id: Uuid, limit: i64, offset: i64, db: &PgPool) -> Result<Vec<Post>> {
        let rows = sqlx::query_as::<_, Post>(
            "SELECT * FROM forum.posts WHERE topic_id = $1 AND is_deleted = FALSE
             ORDER BY created_at, id LIMIT $2 OFFSET $3",
        )
        .bind(topic_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn count_by_topic(topic_id: Uuid, db: &PgPool) -> Result<i64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM forum.posts WHERE topic_id = $1 AND is_deleted = FALSE",
        )
        .bind(topic_id)
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
    pub async fn create(topic_id: Uuid, forum_id: Uuid, author_id: Uuid, dto: CreatePostDto, db: &PgPool) -> Result<Post> {
        let mut tx = db.begin().await?;

        let post = sqlx::query_as::<_, Post>(
            "INSERT INTO forum.posts (topic_id, forum_id, author_id, body_md, reply_to_post_id, is_first_post)
             VALUES ($1, $2, $3, $4, $5, FALSE) RETURNING *",
        )
        .bind(topic_id)
        .bind(forum_id)
        .bind(author_id)
        .bind(&dto.body_md)
        .bind(dto.reply_to_post_id)
        .fetch_one(&mut *tx)
        .await?;

        aggregates::recompute_topic(&mut tx, topic_id).await?;
        aggregates::recompute_forum(&mut tx, forum_id).await?;
        RankService::bump_post_count(&mut tx, author_id, 1).await?;

        tx.commit().await?;
        Ok(post)
    }

    pub async fn update(id: Uuid, user: &ForumUser, dto: UpdatePostDto, db: &PgPool) -> Result<Post> {
        let post = Self::get(id, db).await?;
        let perms = PermissionService::effective(post.forum_id, user, db).await?;
        if post.author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }
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
        .fetch_one(db)
        .await?;
        Ok(row)
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
        RankService::bump_post_count(&mut tx, post.author_id, -1).await?;
        tx.commit().await?;
        Ok(())
    }
}
