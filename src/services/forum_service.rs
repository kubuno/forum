use sqlx::{PgPool, Postgres, QueryBuilder};
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
    pub async fn list(user: &ForumUser, db: &PgPool) -> Result<Vec<Forum>> {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT f.* FROM forum.forums f WHERE ");
        PermissionService::push_visible_forum(&mut qb, "f.id", user);
        qb.push(" ORDER BY f.position, f.name");
        let rows = qb.build_query_as::<Forum>().fetch_all(db).await?;
        Ok(rows)
    }

    pub async fn list_by_category(user: &ForumUser, category_id: Uuid, db: &PgPool) -> Result<Vec<Forum>> {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT f.* FROM forum.forums f WHERE f.category_id = ");
        qb.push_bind(category_id).push(" AND ");
        PermissionService::push_visible_forum(&mut qb, "f.id", user);
        qb.push(" ORDER BY f.position, f.name");
        let rows = qb.build_query_as::<Forum>().fetch_all(db).await?;
        Ok(rows)
    }

    pub async fn get(id: Uuid, db: &PgPool) -> Result<Forum> {
        sqlx::query_as::<_, Forum>("SELECT * FROM forum.forums WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("Forum {id}")))
    }

    pub async fn create(dto: CreateForumDto, db: &PgPool) -> Result<Forum> {
        let row = sqlx::query_as::<_, Forum>(
            "INSERT INTO forum.forums (category_id, parent_forum_id, name, description, position)
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(dto.category_id)
        .bind(dto.parent_forum_id)
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(dto.position)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn update(id: Uuid, dto: UpdateForumDto, db: &PgPool) -> Result<Forum> {
        let row = sqlx::query_as::<_, Forum>(
            "UPDATE forum.forums SET
                category_id = COALESCE($2, category_id),
                name        = COALESCE($3, name),
                description = COALESCE($4, description),
                position    = COALESCE($5, position),
                is_locked   = COALESCE($6, is_locked),
                color       = COALESCE($7, color),
                icon        = COALESCE($8, icon),
                is_readonly = COALESCE($9, is_readonly),
                rules_md    = COALESCE($10, rules_md)
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(dto.category_id)
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(dto.position)
        .bind(dto.is_locked)
        .bind(&dto.color)
        .bind(&dto.icon)
        .bind(dto.is_readonly)
        .bind(&dto.rules_md)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ForumError::NotFound(format!("Forum {id}")))?;
        Ok(row)
    }

    pub async fn delete(id: Uuid, db: &PgPool) -> Result<()> {
        let res = sqlx::query("DELETE FROM forum.forums WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        if res.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("Forum {id}")));
        }
        Ok(())
    }

    /// Reorders forums: sets `position` to each id's index in `ids`, all in one
    /// transaction. Ids not present in the list are left untouched, so the
    /// caller can reorder a single category's subset without disturbing the
    /// position of every other forum in the tree.
    pub async fn reorder(ids: &[Uuid], db: &PgPool) -> Result<()> {
        let mut tx = db.begin().await?;
        for (index, id) in ids.iter().enumerate() {
            let res = sqlx::query("UPDATE forum.forums SET position = $2 WHERE id = $1")
                .bind(id)
                .bind(index as i32)
                .execute(&mut *tx)
                .await
                .inspect_err(|e| tracing::error!(error = %e, %id, "Reordering forum"))?;
            if res.rows_affected() == 0 {
                tx.rollback().await?;
                return Err(ForumError::NotFound(format!("Forum {id}")));
            }
        }
        tx.commit().await?;
        Ok(())
    }
}
