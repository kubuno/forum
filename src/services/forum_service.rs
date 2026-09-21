use kubuno_db::{new_id, params, DbPool, DbQueryBuilder};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::forum::{CreateForumDto, Forum, UpdateForumDto},
    services::permission_service::PermissionService,
};

pub struct ForumService;

impl ForumService {
    /// Lists forums the caller may see (SEC-14): a forum whose `role = 'user'`
    /// permission denies `can_view` is hidden from non-moderators.
    pub async fn list(user: &ForumUser, db: &DbPool) -> Result<Vec<Forum>> {
        let mut qb = DbQueryBuilder::new(db.backend(), "SELECT f.* FROM forum.forums f WHERE ");
        PermissionService::push_visible_forum(&mut qb, "f.id", user);
        qb.push(" ORDER BY f.position, f.name");
        let rows = qb.fetch_all_as::<Forum>(db).await?;
        Ok(rows)
    }

    pub async fn list_by_category(user: &ForumUser, category_id: Uuid, db: &DbPool) -> Result<Vec<Forum>> {
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "SELECT f.* FROM forum.forums f WHERE f.category_id = ",
        );
        qb.push_bind(category_id).push(" AND ");
        PermissionService::push_visible_forum(&mut qb, "f.id", user);
        qb.push(" ORDER BY f.position, f.name");
        let rows = qb.fetch_all_as::<Forum>(db).await?;
        Ok(rows)
    }

    pub async fn get(id: Uuid, db: &DbPool) -> Result<Forum> {
        db.fetch_optional_as::<Forum>("SELECT * FROM forum.forums WHERE id = $1", params![id])
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("Forum {id}")))
    }

    pub async fn create(dto: CreateForumDto, db: &DbPool) -> Result<Forum> {
        let id = new_id();
        db.execute(
            "INSERT INTO forum.forums (id, category_id, parent_forum_id, name, description, position) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            params![
                id,
                dto.category_id,
                dto.parent_forum_id,
                dto.name,
                dto.description,
                dto.position
            ],
        )
        .await?;
        Self::get(id, db).await
    }

    pub async fn update(id: Uuid, dto: UpdateForumDto, db: &DbPool) -> Result<Forum> {
        let affected = db
            .execute(
                "UPDATE forum.forums SET \
                    category_id = COALESCE($1, category_id), \
                    name        = COALESCE($2, name), \
                    description = COALESCE($3, description), \
                    position    = COALESCE($4, position), \
                    is_locked   = COALESCE($5, is_locked), \
                    color       = COALESCE($6, color), \
                    icon        = COALESCE($7, icon), \
                    is_readonly = COALESCE($8, is_readonly), \
                    rules_md    = COALESCE($9, rules_md) \
                 WHERE id = $10",
                params![
                    dto.category_id,
                    dto.name,
                    dto.description,
                    dto.position,
                    dto.is_locked,
                    dto.color,
                    dto.icon,
                    dto.is_readonly,
                    dto.rules_md,
                    id
                ],
            )
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Forum {id}")));
        }
        Self::get(id, db).await
    }

    pub async fn delete(id: Uuid, db: &DbPool) -> Result<()> {
        let affected = db
            .execute("DELETE FROM forum.forums WHERE id = $1", params![id])
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Forum {id}")));
        }
        Ok(())
    }

    /// Reorders forums: sets `position` to each id's index in `ids`, all in one
    /// transaction. Ids not present in the list are left untouched.
    pub async fn reorder(ids: &[Uuid], db: &DbPool) -> Result<()> {
        let mut tx = db.begin().await?;
        for (index, id) in ids.iter().enumerate() {
            let affected = tx
                .execute(
                    "UPDATE forum.forums SET position = $1 WHERE id = $2",
                    params![index as i32, id],
                )
                .await
                .inspect_err(|e| tracing::error!(error = %e, %id, "Reordering forum"))?;
            if affected == 0 {
                tx.rollback().await?;
                return Err(ForumError::NotFound(format!("Forum {id}")));
            }
        }
        tx.commit().await?;
        Ok(())
    }
}
