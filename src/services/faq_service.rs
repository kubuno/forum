//! Editable FAQ (phpBB-style): a short admin-curated list of question/answer
//! pairs, read by every member and written only through the admin console
//! (see `handlers/faq.rs`, gated by `PermissionService::assert_admin`).

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::faq::{CreateFaqDto, FaqEntry, UpdateFaqDto},
};

pub struct FaqService;

impl FaqService {
    pub async fn list(db: &PgPool) -> Result<Vec<FaqEntry>> {
        let rows = sqlx::query_as::<_, FaqEntry>(
            "SELECT * FROM forum.faq_entries ORDER BY position, created_at",
        )
        .fetch_all(db)
        .await
        .inspect_err(|e| tracing::error!(error = %e, "Listing FAQ entries"))?;
        Ok(rows)
    }

    pub async fn create(dto: CreateFaqDto, db: &PgPool) -> Result<FaqEntry> {
        let row = sqlx::query_as::<_, FaqEntry>(
            "INSERT INTO forum.faq_entries (question, answer_md, position)
             VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(dto.question.trim())
        .bind(dto.answer_md.trim())
        .bind(dto.position)
        .fetch_one(db)
        .await
        .inspect_err(|e| tracing::error!(error = %e, "Creating a FAQ entry"))?;
        Ok(row)
    }

    /// Merges only the fields the caller sent — `COALESCE` against the
    /// existing row — and always bumps `updated_at`.
    pub async fn update(id: Uuid, dto: UpdateFaqDto, db: &PgPool) -> Result<FaqEntry> {
        let row = sqlx::query_as::<_, FaqEntry>(
            "UPDATE forum.faq_entries SET
                question   = COALESCE($2, question),
                answer_md  = COALESCE($3, answer_md),
                position   = COALESCE($4, position),
                updated_at = NOW()
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(dto.question.as_deref().map(str::trim))
        .bind(dto.answer_md.as_deref().map(str::trim))
        .bind(dto.position)
        .fetch_optional(db)
        .await
        .inspect_err(|e| tracing::error!(error = %e, %id, "Updating a FAQ entry"))?
        .ok_or_else(|| ForumError::NotFound(format!("FAQ entry {id}")))?;
        Ok(row)
    }

    pub async fn delete(id: Uuid, db: &PgPool) -> Result<()> {
        let r = sqlx::query("DELETE FROM forum.faq_entries WHERE id = $1")
            .bind(id)
            .execute(db)
            .await
            .inspect_err(|e| tracing::error!(error = %e, %id, "Deleting a FAQ entry"))?;
        if r.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("FAQ entry {id}")));
        }
        Ok(())
    }
}
