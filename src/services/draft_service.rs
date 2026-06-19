use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::draft::{Draft, SaveDraftDto},
};

pub struct DraftService;

impl DraftService {
    /// Upserts the single draft for a given context (a forum = new topic, or a
    /// topic = reply). An empty body deletes the draft.
    pub async fn save(user_id: Uuid, dto: SaveDraftDto, db: &PgPool) -> Result<Option<Draft>> {
        if dto.topic_id.is_none() && dto.forum_id.is_none() {
            return Err(ForumError::Validation("forum_id or topic_id required".into()));
        }
        if dto.body_md.trim().is_empty() && dto.title.as_deref().unwrap_or("").trim().is_empty() {
            // Nothing worth keeping — clear any existing draft for this context.
            Self::clear(user_id, dto.forum_id, dto.topic_id, db).await?;
            return Ok(None);
        }

        let draft = if let Some(topic_id) = dto.topic_id {
            sqlx::query_as::<_, Draft>(
                "INSERT INTO forum.drafts (user_id, topic_id, body_md, updated_at)
                 VALUES ($1, $2, $3, NOW())
                 ON CONFLICT (user_id, topic_id) WHERE topic_id IS NOT NULL
                 DO UPDATE SET body_md = EXCLUDED.body_md, updated_at = NOW()
                 RETURNING *",
            )
            .bind(user_id)
            .bind(topic_id)
            .bind(&dto.body_md)
            .fetch_one(db)
            .await?
        } else {
            let forum_id = dto.forum_id.unwrap();
            sqlx::query_as::<_, Draft>(
                "INSERT INTO forum.drafts (user_id, forum_id, title, body_md, updated_at)
                 VALUES ($1, $2, $3, $4, NOW())
                 ON CONFLICT (user_id, forum_id) WHERE forum_id IS NOT NULL AND topic_id IS NULL
                 DO UPDATE SET title = EXCLUDED.title, body_md = EXCLUDED.body_md, updated_at = NOW()
                 RETURNING *",
            )
            .bind(user_id)
            .bind(forum_id)
            .bind(&dto.title)
            .bind(&dto.body_md)
            .fetch_one(db)
            .await?
        };
        Ok(Some(draft))
    }

    async fn clear(user_id: Uuid, forum_id: Option<Uuid>, topic_id: Option<Uuid>, db: &PgPool) -> Result<()> {
        if let Some(t) = topic_id {
            sqlx::query("DELETE FROM forum.drafts WHERE user_id = $1 AND topic_id = $2")
                .bind(user_id).bind(t).execute(db).await?;
        } else if let Some(f) = forum_id {
            sqlx::query("DELETE FROM forum.drafts WHERE user_id = $1 AND forum_id = $2 AND topic_id IS NULL")
                .bind(user_id).bind(f).execute(db).await?;
        }
        Ok(())
    }

    pub async fn list(user_id: Uuid, db: &PgPool) -> Result<Vec<Draft>> {
        let rows = sqlx::query_as::<_, Draft>(
            "SELECT * FROM forum.drafts WHERE user_id = $1 ORDER BY updated_at DESC",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn delete(user_id: Uuid, id: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM forum.drafts WHERE id = $1 AND user_id = $2")
            .bind(id).bind(user_id).execute(db).await?;
        Ok(())
    }
}
