use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::forum::{CreateForumDto, Forum, UpdateForumDto},
};

pub struct ForumService;

impl ForumService {
    pub async fn list(db: &PgPool) -> Result<Vec<Forum>> {
        let rows = sqlx::query_as::<_, Forum>(
            "SELECT * FROM forum.forums ORDER BY position, name",
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn list_by_category(category_id: Uuid, db: &PgPool) -> Result<Vec<Forum>> {
        let rows = sqlx::query_as::<_, Forum>(
            "SELECT * FROM forum.forums WHERE category_id = $1 ORDER BY position, name",
        )
        .bind(category_id)
        .fetch_all(db)
        .await?;
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
                is_locked   = COALESCE($6, is_locked)
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(dto.category_id)
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(dto.position)
        .bind(dto.is_locked)
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
}
