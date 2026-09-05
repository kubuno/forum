use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::attachment::{Attachment, CreateAttachmentDto},
    services::permission_service::PermissionService,
};

pub struct AttachmentService;

impl AttachmentService {
    pub async fn list(post_id: Uuid, db: &PgPool) -> Result<Vec<Attachment>> {
        let rows = sqlx::query_as::<_, Attachment>(
            "SELECT * FROM forum.attachments WHERE post_id = $1 ORDER BY created_at",
        )
        .bind(post_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn create(post_id: Uuid, user: &ForumUser, dto: CreateAttachmentDto, db: &PgPool) -> Result<Attachment> {
        // Only the post author (or a moderator/admin) may attach files to a post.
        let post: Option<(Uuid, Uuid)> =
            sqlx::query_as("SELECT author_id, forum_id FROM forum.posts WHERE id = $1")
                .bind(post_id)
                .fetch_optional(db)
                .await?;
        let (author_id, forum_id) = post.ok_or_else(|| ForumError::NotFound(format!("Post {post_id}")))?;
        let perms = PermissionService::effective(forum_id, user, db).await?;
        if author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }
        // The forum's per-role permission may forbid attachments; it was resolved
        // but never enforced before (SEC-04). Moderators/admins keep can_attach.
        if !perms.can_attach {
            return Err(ForumError::Forbidden);
        }
        // Cap the number of attachments on a single post so one message cannot
        // accumulate an unbounded list of file references.
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM forum.attachments WHERE post_id = $1")
            .bind(post_id)
            .fetch_one(db)
            .await?;
        if count >= 10 {
            return Err(ForumError::Conflict(
                "this message already has the maximum number of attachments".into(),
            ));
        }
        // NOTE (SEC-04, remaining M-effort work): filename / mime_type / size are
        // still trusted from the client, and file_id is not yet re-checked against
        // the Drive module for ownership. That resolution via an authenticated
        // internal Drive call is a dedicated follow-up before attachments ship
        // to untrusted users.
        let row = sqlx::query_as::<_, Attachment>(
            "INSERT INTO forum.attachments (post_id, file_id, filename, mime_type, size_bytes)
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(post_id)
        .bind(dto.file_id)
        .bind(&dto.filename)
        .bind(&dto.mime_type)
        .bind(dto.size_bytes)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn delete(id: Uuid, user: &ForumUser, db: &PgPool) -> Result<()> {
        let row: Option<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT p.author_id, p.forum_id
               FROM forum.attachments a JOIN forum.posts p ON p.id = a.post_id
              WHERE a.id = $1",
        )
        .bind(id)
        .fetch_optional(db)
        .await?;
        let (author_id, forum_id) = row.ok_or_else(|| ForumError::NotFound(format!("Attachment {id}")))?;
        let perms = PermissionService::effective(forum_id, user, db).await?;
        if author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }
        sqlx::query("DELETE FROM forum.attachments WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }
}
