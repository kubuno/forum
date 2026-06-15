use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::category::{Category, CreateCategoryDto, UpdateCategoryDto},
};

pub struct CategoryService;

impl CategoryService {
    pub async fn list(db: &PgPool) -> Result<Vec<Category>> {
        let rows = sqlx::query_as::<_, Category>(
            "SELECT * FROM forum.categories ORDER BY position, name",
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn get(id: Uuid, db: &PgPool) -> Result<Category> {
        sqlx::query_as::<_, Category>("SELECT * FROM forum.categories WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("Category {id}")))
    }

    pub async fn create(dto: CreateCategoryDto, db: &PgPool) -> Result<Category> {
        let row = sqlx::query_as::<_, Category>(
            "INSERT INTO forum.categories (name, description, position)
             VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(dto.position)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn update(id: Uuid, dto: UpdateCategoryDto, db: &PgPool) -> Result<Category> {
        let row = sqlx::query_as::<_, Category>(
            "UPDATE forum.categories SET
                name        = COALESCE($2, name),
                description = COALESCE($3, description),
                position    = COALESCE($4, position)
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(dto.position)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ForumError::NotFound(format!("Category {id}")))?;
        Ok(row)
    }

    pub async fn delete(id: Uuid, db: &PgPool) -> Result<()> {
        let res = sqlx::query("DELETE FROM forum.categories WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        if res.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("Category {id}")));
        }
        Ok(())
    }
}
