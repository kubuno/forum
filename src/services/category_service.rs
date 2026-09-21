use kubuno_db::dialect::SqlType;
use kubuno_db::{new_id, params, DbPool, DbQueryBuilder};
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
    pub async fn list(user: &ForumUser, db: &DbPool) -> Result<Vec<Category>> {
        if user.is_admin() {
            let rows = db
                .fetch_all_as::<Category>(
                    "SELECT * FROM forum.categories ORDER BY position, name",
                    params![],
                )
                .await?;
            return Ok(rows);
        }
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "SELECT c.* FROM forum.categories c WHERE EXISTS \
             (SELECT 1 FROM forum.forums f WHERE f.category_id = c.id AND ",
        );
        PermissionService::push_visible_forum(&mut qb, "f.id", user);
        qb.push(") ORDER BY c.position, c.name");
        let rows = qb.fetch_all_as::<Category>(db).await?;
        Ok(rows)
    }

    pub async fn get(id: Uuid, db: &DbPool) -> Result<Category> {
        db.fetch_optional_as::<Category>(
            "SELECT * FROM forum.categories WHERE id = $1",
            params![id],
        )
        .await?
        .ok_or_else(|| ForumError::NotFound(format!("Category {id}")))
    }

    /// A non-admin may only fetch a category that still holds a forum they can
    /// view; otherwise it is reported as absent, never as forbidden (SEC-14).
    pub async fn assert_visible(user: &ForumUser, id: Uuid, db: &DbPool) -> Result<()> {
        if user.is_admin() {
            return Ok(());
        }
        let one = db.backend().cast("1", SqlType::BigInt);
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            format!("SELECT {one} FROM forum.forums f WHERE f.category_id = "),
        );
        qb.push_bind(id).push(" AND ");
        PermissionService::push_visible_forum(&mut qb, "f.id", user);
        qb.push(" LIMIT 1");
        let visible: Option<i64> = qb.fetch_optional_scalar(db).await?;
        if visible.is_some() {
            Ok(())
        } else {
            Err(ForumError::NotFound(format!("Category {id}")))
        }
    }

    pub async fn create(dto: CreateCategoryDto, db: &DbPool) -> Result<Category> {
        let id = new_id();
        db.execute(
            "INSERT INTO forum.categories (id, name, description, position) \
             VALUES ($1, $2, $3, $4)",
            params![id, dto.name, dto.description, dto.position],
        )
        .await?;
        Self::get(id, db).await
    }

    pub async fn update(id: Uuid, dto: UpdateCategoryDto, db: &DbPool) -> Result<Category> {
        let affected = db
            .execute(
                "UPDATE forum.categories SET \
                    name        = COALESCE($1, name), \
                    description = COALESCE($2, description), \
                    position    = COALESCE($3, position) \
                 WHERE id = $4",
                params![dto.name, dto.description, dto.position, id],
            )
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Category {id}")));
        }
        Self::get(id, db).await
    }

    pub async fn delete(id: Uuid, db: &DbPool) -> Result<()> {
        let affected = db
            .execute("DELETE FROM forum.categories WHERE id = $1", params![id])
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Category {id}")));
        }
        Ok(())
    }
}
