use kubuno_db::dialect::Assign;
use kubuno_db::{new_id, params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::draft::{Draft, SaveDraftDto},
};

pub struct DraftService;

impl DraftService {
    /// Upserts the single draft for a given context (a forum = new topic, or a
    /// topic = reply). An empty body deletes the draft.
    pub async fn save(user_id: Uuid, dto: SaveDraftDto, db: &DbPool) -> Result<Option<Draft>> {
        if dto.topic_id.is_none() && dto.forum_id.is_none() {
            return Err(ForumError::Validation("forum_id or topic_id required".into()));
        }
        if dto.body_md.trim().is_empty() && dto.title.as_deref().unwrap_or("").trim().is_empty() {
            // Nothing worth keeping — clear any existing draft for this context.
            Self::clear(user_id, dto.forum_id, dto.topic_id, db).await?;
            return Ok(None);
        }

        let b = db.backend();
        let now = chrono::Utc::now();
        // The former partial-index conflict targets are now plain UNIQUE
        // constraints (see migration 000021), so the upsert names the pair.
        let draft = if let Some(topic_id) = dto.topic_id {
            let sql = format!(
                "INSERT INTO forum.drafts (id, user_id, topic_id, body_md, updated_at) \
                 VALUES ($1, $2, $3, $4, $5){}",
                b.upsert(
                    "drafts",
                    &["user_id", "topic_id"],
                    &[Assign::Incoming("body_md"), Assign::Incoming("updated_at")],
                )
            );
            db.execute(&sql, params![new_id(), user_id, topic_id, dto.body_md, now])
                .await?;
            db.fetch_one_as::<Draft>(
                "SELECT * FROM forum.drafts WHERE user_id = $1 AND topic_id = $2",
                params![user_id, topic_id],
            )
            .await?
        } else {
            let forum_id = dto.forum_id.unwrap();
            let sql = format!(
                "INSERT INTO forum.drafts (id, user_id, forum_id, title, body_md, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6){}",
                b.upsert(
                    "drafts",
                    &["user_id", "forum_id"],
                    &[
                        Assign::Incoming("title"),
                        Assign::Incoming("body_md"),
                        Assign::Incoming("updated_at"),
                    ],
                )
            );
            db.execute(
                &sql,
                params![new_id(), user_id, forum_id, dto.title.clone(), dto.body_md, now],
            )
            .await?;
            db.fetch_one_as::<Draft>(
                "SELECT * FROM forum.drafts WHERE user_id = $1 AND forum_id = $2 AND topic_id IS NULL",
                params![user_id, forum_id],
            )
            .await?
        };
        Ok(Some(draft))
    }

    async fn clear(user_id: Uuid, forum_id: Option<Uuid>, topic_id: Option<Uuid>, db: &DbPool) -> Result<()> {
        if let Some(t) = topic_id {
            db.execute(
                "DELETE FROM forum.drafts WHERE user_id = $1 AND topic_id = $2",
                params![user_id, t],
            )
            .await?;
        } else if let Some(f) = forum_id {
            db.execute(
                "DELETE FROM forum.drafts WHERE user_id = $1 AND forum_id = $2 AND topic_id IS NULL",
                params![user_id, f],
            )
            .await?;
        }
        Ok(())
    }

    pub async fn list(user_id: Uuid, db: &DbPool) -> Result<Vec<Draft>> {
        let rows = db
            .fetch_all_as::<Draft>(
                "SELECT * FROM forum.drafts WHERE user_id = $1 ORDER BY updated_at DESC",
                params![user_id],
            )
            .await?;
        Ok(rows)
    }

    pub async fn delete(user_id: Uuid, id: Uuid, db: &DbPool) -> Result<()> {
        db.execute(
            "DELETE FROM forum.drafts WHERE id = $1 AND user_id = $2",
            params![id, user_id],
        )
        .await?;
        Ok(())
    }
}
