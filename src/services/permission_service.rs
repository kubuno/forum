use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::permission::{Permission, SetPermissionDto},
};

/// Effective capabilities of a user on a given forum.
pub struct EffectivePerms {
    pub can_view:   bool,
    pub can_post:   bool,
    pub can_reply:  bool,
    pub can_attach: bool,
    pub is_moderator: bool,
    pub is_admin:     bool,
}

pub struct PermissionService;

impl PermissionService {
    /// True when the user moderates the given forum.
    pub async fn is_moderator(forum_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<bool> {
        let exists: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM forum.moderators WHERE forum_id = $1 AND user_id = $2",
        )
        .bind(forum_id)
        .bind(user_id)
        .fetch_optional(db)
        .await?;
        Ok(exists.is_some())
    }

    /// Resolve what a user can do on a forum. Admins and moderators are unrestricted.
    /// For regular users we read the per-forum 'user' permission row, defaulting to
    /// fully permissive when none is configured.
    pub async fn effective(forum_id: Uuid, user: &ForumUser, db: &PgPool) -> Result<EffectivePerms> {
        let is_admin = user.is_admin();
        let is_moderator = if is_admin {
            true
        } else {
            Self::is_moderator(forum_id, user.id, db).await?
        };

        if is_admin || is_moderator {
            return Ok(EffectivePerms {
                can_view: true, can_post: true, can_reply: true, can_attach: true,
                is_moderator, is_admin,
            });
        }

        let row: Option<Permission> = sqlx::query_as::<_, Permission>(
            "SELECT * FROM forum.permissions WHERE forum_id = $1 AND role = 'user'",
        )
        .bind(forum_id)
        .fetch_optional(db)
        .await?;

        Ok(match row {
            Some(p) => EffectivePerms {
                can_view: p.can_view, can_post: p.can_post,
                can_reply: p.can_reply, can_attach: p.can_attach,
                is_moderator, is_admin,
            },
            None => EffectivePerms {
                can_view: true, can_post: true, can_reply: true, can_attach: true,
                is_moderator, is_admin,
            },
        })
    }

    pub async fn assert_can_view(forum_id: Uuid, user: &ForumUser, db: &PgPool) -> Result<EffectivePerms> {
        let p = Self::effective(forum_id, user, db).await?;
        if !p.can_view {
            return Err(ForumError::Forbidden);
        }
        Ok(p)
    }

    /// Only platform admins manage categories, forums, ranks and permissions.
    pub fn assert_admin(user: &ForumUser) -> Result<()> {
        if user.is_admin() { Ok(()) } else { Err(ForumError::Forbidden) }
    }

    // ── CRUD on per-forum permissions (admin only) ─────────────────────────────

    pub async fn list(forum_id: Uuid, db: &PgPool) -> Result<Vec<Permission>> {
        let rows = sqlx::query_as::<_, Permission>(
            "SELECT * FROM forum.permissions WHERE forum_id = $1 ORDER BY role",
        )
        .bind(forum_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn set(forum_id: Uuid, dto: SetPermissionDto, db: &PgPool) -> Result<Permission> {
        if !matches!(dto.role.as_str(), "guest" | "user" | "moderator") {
            return Err(ForumError::Validation(format!("invalid role: {}", dto.role)));
        }
        let row = sqlx::query_as::<_, Permission>(
            "INSERT INTO forum.permissions (forum_id, role, can_view, can_post, can_reply, can_attach)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (forum_id, role) DO UPDATE
                SET can_view = EXCLUDED.can_view, can_post = EXCLUDED.can_post,
                    can_reply = EXCLUDED.can_reply, can_attach = EXCLUDED.can_attach
             RETURNING *",
        )
        .bind(forum_id)
        .bind(&dto.role)
        .bind(dto.can_view)
        .bind(dto.can_post)
        .bind(dto.can_reply)
        .bind(dto.can_attach)
        .fetch_one(db)
        .await?;
        Ok(row)
    }
}
