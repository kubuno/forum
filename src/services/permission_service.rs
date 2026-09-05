use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::permission::{GroupPermission, Permission, SetGroupPermissionDto, SetPermissionDto},
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

        // Baseline from the role rule (default: all allowed when no row exists).
        let (mut can_view, mut can_post, mut can_reply, mut can_attach) = match row {
            Some(p) => (p.can_view, p.can_post, p.can_reply, p.can_attach),
            None => (true, true, true, true),
        };

        // Additive per-group grants: any group the caller belongs to may widen
        // access. `bool_or` yields NULL (→ None → false) when no grant matches,
        // so this can only turn a permission ON, never off.
        if !user.group_ids.is_empty() {
            let g: (Option<bool>, Option<bool>, Option<bool>, Option<bool>) = sqlx::query_as(
                "SELECT bool_or(can_view), bool_or(can_post), bool_or(can_reply), bool_or(can_attach) \
                   FROM forum.group_permissions WHERE forum_id = $1 AND group_id = ANY($2)",
            )
            .bind(forum_id)
            .bind(&user.group_ids)
            .fetch_one(db)
            .await?;
            can_view   = can_view   || g.0.unwrap_or(false);
            can_post   = can_post   || g.1.unwrap_or(false);
            can_reply  = can_reply  || g.2.unwrap_or(false);
            can_attach = can_attach || g.3.unwrap_or(false);
        }

        Ok(EffectivePerms { can_view, can_post, can_reply, can_attach, is_moderator, is_admin })
    }

    /// Appends a boolean predicate (no leading `AND`) restricting a query to the
    /// forums the user may see. Single source of truth shared by every listing and
    /// search path, so visibility can never diverge between them (SEC-01/14).
    ///
    /// Rules mirror `effective`: platform admins see everything; a per-forum
    /// moderator sees the forums they moderate; everyone else is denied any forum
    /// whose `role = 'user'` permission row sets `can_view = FALSE`.
    ///
    /// `forum_col` is a caller-controlled SQL column expression (e.g. `f.id`,
    /// `p.forum_id`) — never user input — so it is safe to inline as raw SQL.
    pub fn push_visible_forum(qb: &mut QueryBuilder<'_, Postgres>, forum_col: &str, user: &ForumUser) {
        if user.is_admin() {
            qb.push("TRUE");
            return;
        }
        qb.push("(EXISTS (SELECT 1 FROM forum.moderators mo WHERE mo.forum_id = ")
            .push(forum_col)
            .push(" AND mo.user_id = ")
            .push_bind(user.id)
            .push(") OR NOT EXISTS (SELECT 1 FROM forum.permissions pm WHERE pm.forum_id = ")
            .push(forum_col)
            .push(" AND pm.role = 'user' AND pm.can_view = FALSE)");
        // Additive per-group grant: a group the caller belongs to may open an
        // otherwise-restricted forum. This clause can only ADD visibility, never
        // remove it, so it cannot regress the role-based rules above. Skipped
        // entirely when the caller is in no group.
        if !user.group_ids.is_empty() {
            qb.push(" OR EXISTS (SELECT 1 FROM forum.group_permissions gp WHERE gp.forum_id = ")
                .push(forum_col)
                .push(" AND gp.can_view = TRUE AND gp.group_id = ANY(")
                .push_bind(user.group_ids.clone())
                .push("))");
        }
        qb.push(")");
    }

    /// Whether a role's per-forum permission row grants `can_view`, defaulting to
    /// true when no row is configured. Used to decide, without knowing a third
    /// party's role, whether a forum is open to ordinary members at all — e.g.
    /// before notifying a mentioned user (SEC-11).
    pub async fn role_can_view(forum_id: Uuid, role: &str, db: &PgPool) -> Result<bool> {
        let row: Option<bool> = sqlx::query_scalar(
            "SELECT can_view FROM forum.permissions WHERE forum_id = $1 AND role = $2",
        )
        .bind(forum_id)
        .bind(role)
        .fetch_optional(db)
        .await?;
        Ok(row.unwrap_or(true))
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

    // ── CRUD on per-GROUP permissions (admin only) ─────────────────────────────

    pub async fn list_group(forum_id: Uuid, db: &PgPool) -> Result<Vec<GroupPermission>> {
        let rows = sqlx::query_as::<_, GroupPermission>(
            "SELECT * FROM forum.group_permissions WHERE forum_id = $1 ORDER BY group_id",
        )
        .bind(forum_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Upserts a per-group grant. When every flag is false the grant is deleted
    /// instead of stored, so an all-off row never lingers as a no-op.
    pub async fn set_group(forum_id: Uuid, dto: SetGroupPermissionDto, db: &PgPool) -> Result<()> {
        if !dto.can_view && !dto.can_post && !dto.can_reply && !dto.can_attach {
            sqlx::query("DELETE FROM forum.group_permissions WHERE forum_id = $1 AND group_id = $2")
                .bind(forum_id)
                .bind(dto.group_id)
                .execute(db)
                .await?;
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO forum.group_permissions (forum_id, group_id, can_view, can_post, can_reply, can_attach)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (forum_id, group_id) DO UPDATE
                SET can_view = EXCLUDED.can_view, can_post = EXCLUDED.can_post,
                    can_reply = EXCLUDED.can_reply, can_attach = EXCLUDED.can_attach",
        )
        .bind(forum_id)
        .bind(dto.group_id)
        .bind(dto.can_view)
        .bind(dto.can_post)
        .bind(dto.can_reply)
        .bind(dto.can_attach)
        .execute(db)
        .await?;
        Ok(())
    }
}
