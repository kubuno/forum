use kubuno_db::{new_id, params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::attachment::{Attachment, CreateAttachmentDto},
    services::permission_service::PermissionService,
};

pub struct AttachmentService;

impl AttachmentService {
    pub async fn list(post_id: Uuid, db: &DbPool) -> Result<Vec<Attachment>> {
        let rows = db
            .fetch_all_as::<Attachment>(
                "SELECT * FROM forum.attachments WHERE post_id = $1 ORDER BY created_at",
                params![post_id],
            )
            .await?;
        Ok(rows)
    }

    pub async fn create(post_id: Uuid, user: &ForumUser, dto: CreateAttachmentDto, db: &DbPool) -> Result<Attachment> {
        // Only the post author (or a moderator/admin) may attach files to a post.
        let post: Option<(Uuid, Uuid)> = db
            .fetch_optional_as::<(Uuid, Uuid)>(
                "SELECT author_id, forum_id FROM forum.posts WHERE id = $1",
                params![post_id],
            )
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
        let count: i64 = db
            .fetch_scalar(
                "SELECT COUNT(*) FROM forum.attachments WHERE post_id = $1",
                params![post_id],
            )
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
        let id = new_id();
        db.execute(
            "INSERT INTO forum.attachments (id, post_id, file_id, filename, mime_type, size_bytes) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            params![id, post_id, dto.file_id, dto.filename, dto.mime_type, dto.size_bytes],
        )
        .await?;
        db.fetch_one_as::<Attachment>(
            "SELECT * FROM forum.attachments WHERE id = $1",
            params![id],
        )
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, user: &ForumUser, db: &DbPool) -> Result<()> {
        let row: Option<(Uuid, Uuid)> = db
            .fetch_optional_as::<(Uuid, Uuid)>(
                "SELECT p.author_id, p.forum_id \
                   FROM forum.attachments a JOIN forum.posts p ON p.id = a.post_id \
                  WHERE a.id = $1",
                params![id],
            )
            .await?;
        let (author_id, forum_id) = row.ok_or_else(|| ForumError::NotFound(format!("Attachment {id}")))?;
        let perms = PermissionService::effective(forum_id, user, db).await?;
        if author_id != user.id && !perms.is_admin && !perms.is_moderator {
            return Err(ForumError::Forbidden);
        }
        db.execute("DELETE FROM forum.attachments WHERE id = $1", params![id]).await?;
        Ok(())
    }
}
