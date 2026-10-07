use kubuno_db::dialect::{Assign, SqlType};
use kubuno_db::{new_id, params, DbPool, DbQueryBuilder};
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

/// The four additive per-group grant flags, read to fold into a user's baseline.
#[derive(sqlx::FromRow)]
struct GrantFlags {
    can_view:   bool,
    can_post:   bool,
    can_reply:  bool,
    can_attach: bool,
}

pub struct PermissionService;

impl PermissionService {
    /// True when the user moderates the given forum.
    pub async fn is_moderator(forum_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<bool> {
        // `1` is cast to one portable width (int4 on PostgreSQL, BIGINT on MySQL,
        // dynamic on SQLite) so the existence probe decodes the same everywhere.
        let sql = format!(
            "SELECT {} FROM forum.moderators WHERE forum_id = $1 AND user_id = $2",
            db.backend().cast("1", SqlType::BigInt)
        );
        let exists: Option<i64> = db.fetch_optional_scalar(&sql, params![forum_id, user_id]).await?;
        Ok(exists.is_some())
    }

    /// Resolve what a user can do on a forum. Admins and moderators are unrestricted.
    /// For regular users we read the per-forum 'user' permission row, defaulting to
    /// fully permissive when none is configured.
    pub async fn effective(forum_id: Uuid, user: &ForumUser, db: &DbPool) -> Result<EffectivePerms> {
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

        let row: Option<Permission> = db
            .fetch_optional_as::<Permission>(
                "SELECT * FROM forum.permissions WHERE forum_id = $1 AND role = 'user'",
                params![forum_id],
            )
            .await?;

        // Baseline from the role rule (default: all allowed when no row exists).
        let (mut can_view, mut can_post, mut can_reply, mut can_attach) = match row {
            Some(p) => (p.can_view, p.can_post, p.can_reply, p.can_attach),
            None => (true, true, true, true),
        };

        // Additive per-group grants: any group the caller belongs to may widen
        // access. This can only turn a permission ON, never off. `bool_or` has no
        // portable spelling, so the grant rows are read and OR-ed in Rust.
        if !user.group_ids.is_empty() {
            let mut qb = DbQueryBuilder::new(
                db.backend(),
                "SELECT can_view, can_post, can_reply, can_attach \
                   FROM forum.group_permissions WHERE forum_id = ",
            );
            qb.push_bind(forum_id).push(" AND group_id").push_in(user.group_ids.iter().copied());
            let grants: Vec<GrantFlags> = qb.fetch_all_as(db).await?;
            for g in grants {
                can_view   = can_view   || g.can_view;
                can_post   = can_post   || g.can_post;
                can_reply  = can_reply  || g.can_reply;
                can_attach = can_attach || g.can_attach;
            }
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
    /// `forum_col` is a SQL column expression (e.g. `f.id`, `p.forum_id`). It is
    /// typed `&'static str` so the compiler guarantees it can only ever be a
    /// string literal written in this crate, never data from a request.
    pub fn push_visible_forum(
        qb: &mut DbQueryBuilder,
        forum_col: &'static str,
        user: &ForumUser,
    ) {
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
                .push(" AND gp.can_view = TRUE AND gp.group_id")
                .push_in(user.group_ids.iter().copied())
                .push(")");
        }
        qb.push(")");
    }

    /// Whether a role's per-forum permission row grants `can_view`, defaulting to
    /// true when no row is configured. Used to decide, without knowing a third
    /// party's role, whether a forum is open to ordinary members at all — e.g.
    /// before notifying a mentioned user (SEC-11).
    pub async fn role_can_view(forum_id: Uuid, role: &str, db: &DbPool) -> Result<bool> {
        let row: Option<bool> = db
            .fetch_optional_scalar(
                "SELECT can_view FROM forum.permissions WHERE forum_id = $1 AND role = $2",
                params![forum_id, role],
            )
            .await?;
        Ok(row.unwrap_or(true))
    }

    pub async fn assert_can_view(forum_id: Uuid, user: &ForumUser, db: &DbPool) -> Result<EffectivePerms> {
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

    pub async fn list(forum_id: Uuid, db: &DbPool) -> Result<Vec<Permission>> {
        let rows = db
            .fetch_all_as::<Permission>(
                "SELECT * FROM forum.permissions WHERE forum_id = $1 ORDER BY role",
                params![forum_id],
            )
            .await?;
        Ok(rows)
    }

    pub async fn set(forum_id: Uuid, dto: SetPermissionDto, db: &DbPool) -> Result<Permission> {
        if !matches!(dto.role.as_str(), "guest" | "user" | "moderator") {
            return Err(ForumError::Validation(format!("invalid role: {}", dto.role)));
        }
        let b = db.backend();
        let sql = format!(
            "INSERT INTO forum.permissions (id, forum_id, role, can_view, can_post, can_reply, can_attach) \
             VALUES ($1, $2, $3, $4, $5, $6, $7){}",
            b.upsert(
                "permissions",
                &["forum_id", "role"],
                &[
                    Assign::Incoming("can_view"),
                    Assign::Incoming("can_post"),
                    Assign::Incoming("can_reply"),
                    Assign::Incoming("can_attach"),
                ],
            )
        );
        db.execute(
            &sql,
            params![
                new_id(),
                forum_id,
                &dto.role,
                dto.can_view,
                dto.can_post,
                dto.can_reply,
                dto.can_attach
            ],
        )
        .await?;
        db.fetch_one_as::<Permission>(
            "SELECT * FROM forum.permissions WHERE forum_id = $1 AND role = $2",
            params![forum_id, dto.role],
        )
        .await
        .map_err(Into::into)
    }

    // ── CRUD on per-GROUP permissions (admin only) ─────────────────────────────

    pub async fn list_group(forum_id: Uuid, db: &DbPool) -> Result<Vec<GroupPermission>> {
        let rows = db
            .fetch_all_as::<GroupPermission>(
                "SELECT * FROM forum.group_permissions WHERE forum_id = $1 ORDER BY group_id",
                params![forum_id],
            )
            .await?;
        Ok(rows)
    }

    /// Upserts a per-group grant. When every flag is false the grant is deleted
    /// instead of stored, so an all-off row never lingers as a no-op.
    pub async fn set_group(forum_id: Uuid, dto: SetGroupPermissionDto, db: &DbPool) -> Result<()> {
        if !dto.can_view && !dto.can_post && !dto.can_reply && !dto.can_attach {
            db.execute(
                "DELETE FROM forum.group_permissions WHERE forum_id = $1 AND group_id = $2",
                params![forum_id, dto.group_id],
            )
            .await?;
            return Ok(());
        }
        let b = db.backend();
        let sql = format!(
            "INSERT INTO forum.group_permissions (id, forum_id, group_id, can_view, can_post, can_reply, can_attach) \
             VALUES ($1, $2, $3, $4, $5, $6, $7){}",
            b.upsert(
                "group_permissions",
                &["forum_id", "group_id"],
                &[
                    Assign::Incoming("can_view"),
                    Assign::Incoming("can_post"),
                    Assign::Incoming("can_reply"),
                    Assign::Incoming("can_attach"),
                ],
            )
        );
        db.execute(
            &sql,
            params![
                new_id(),
                forum_id,
                dto.group_id,
                dto.can_view,
                dto.can_post,
                dto.can_reply,
                dto.can_attach
            ],
        )
        .await?;
        Ok(())
    }
}
