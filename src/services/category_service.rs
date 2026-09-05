use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::category::{Category, CreateCategoryDto, UpdateCategoryDto},
    services::permission_service::PermissionService,
};

pub struct CategoryService;

impl CategoryService {
    /// Lists categories. Admins see all; everyone else only sees a category that
    /// still holds at least one forum they may view, so a category made only of
    /// restricted forums never leaks its name (SEC-14).
    pub async fn list(user: &ForumUser, db: &PgPool) -> Result<Vec<Category>> {
        if user.is_admin() {
            let rows = sqlx::query_as::<_, Category>(
                "SELECT * FROM forum.categories ORDER BY position, name",
            )
            .fetch_all(db)
            .await?;
            return Ok(rows);
        }
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT c.* FROM forum.categories c WHERE EXISTS \
             (SELECT 1 FROM forum.forums f WHERE f.category_id = c.id AND ",
        );
        PermissionService::push_visible_forum(&mut qb, "f.id", user);
        qb.push(") ORDER BY c.position, c.name");
        let rows = qb.build_query_as::<Category>().fetch_all(db).await?;
        Ok(rows)
    }

    pub async fn get(id: Uuid, db: &PgPool) -> Result<Category> {
        sqlx::query_as::<_, Category>("SELECT * FROM forum.categories WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("Category {id}")))
    }

    /// A non-admin may only fetch a category that still holds a forum they can
    /// view; otherwise it is reported as absent, never as forbidden (SEC-14).
    pub async fn assert_visible(user: &ForumUser, id: Uuid, db: &PgPool) -> Result<()> {
        if user.is_admin() {
            return Ok(());
        }
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT EXISTS (SELECT 1 FROM forum.forums f WHERE f.category_id = ",
        );
        qb.push_bind(id).push(" AND ");
        PermissionService::push_visible_forum(&mut qb, "f.id", user);
        qb.push(")");
        let visible: bool = qb.build_query_scalar().fetch_one(db).await?;
        if visible {
            Ok(())
        } else {
            Err(ForumError::NotFound(format!("Category {id}")))
        }
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
