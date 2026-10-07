//! Editable FAQ: a short admin-curated list of question/answer
//! pairs, read by every member and written only through the admin console
//! (see `handlers/faq.rs`, gated by `PermissionService::assert_admin`).

use kubuno_db::{new_id, params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::faq::{CreateFaqDto, FaqEntry, UpdateFaqDto},
};

pub struct FaqService;

impl FaqService {
    pub async fn list(db: &DbPool) -> Result<Vec<FaqEntry>> {
        let rows = db
            .fetch_all_as::<FaqEntry>(
                "SELECT * FROM forum.faq_entries ORDER BY position, created_at",
                params![],
            )
            .await
            .inspect_err(|e| tracing::error!(error = %e, "Listing FAQ entries"))?;
        Ok(rows)
    }

    pub async fn create(dto: CreateFaqDto, db: &DbPool) -> Result<FaqEntry> {
        let id = new_id();
        db.execute(
            "INSERT INTO forum.faq_entries (id, question, answer_md, position) \
             VALUES ($1, $2, $3, $4)",
            params![id, dto.question.trim(), dto.answer_md.trim(), dto.position],
        )
        .await
        .inspect_err(|e| tracing::error!(error = %e, "Creating a FAQ entry"))?;
        db.fetch_one_as::<FaqEntry>(
            "SELECT * FROM forum.faq_entries WHERE id = $1",
            params![id],
        )
        .await
        .map_err(Into::into)
    }

    /// Merges only the fields the caller sent — `COALESCE` against the
    /// existing row — and always bumps `updated_at`.
    pub async fn update(id: Uuid, dto: UpdateFaqDto, db: &DbPool) -> Result<FaqEntry> {
        let affected = db
            .execute(
                "UPDATE forum.faq_entries SET \
                    question   = COALESCE($1, question), \
                    answer_md  = COALESCE($2, answer_md), \
                    position   = COALESCE($3, position), \
                    updated_at = $4 \
                 WHERE id = $5",
                params![
                    dto.question.as_deref().map(str::trim),
                    dto.answer_md.as_deref().map(str::trim),
                    dto.position,
                    chrono::Utc::now(),
                    id
                ],
            )
            .await
            .inspect_err(|e| tracing::error!(error = %e, %id, "Updating a FAQ entry"))?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("FAQ entry {id}")));
        }
        db.fetch_one_as::<FaqEntry>(
            "SELECT * FROM forum.faq_entries WHERE id = $1",
            params![id],
        )
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, db: &DbPool) -> Result<()> {
        let affected = db
            .execute("DELETE FROM forum.faq_entries WHERE id = $1", params![id])
            .await
            .inspect_err(|e| tracing::error!(error = %e, %id, "Deleting a FAQ entry"))?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("FAQ entry {id}")));
        }
        Ok(())
    }
}
