use std::collections::HashMap;

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::tag::{CreateTagDto, Tag},
    services::aggregates::slugify,
};

pub struct TagService;

impl TagService {
    pub async fn list(db: &PgPool) -> Result<Vec<Tag>> {
        let rows = sqlx::query_as::<_, Tag>(
            "SELECT t.*, (SELECT COUNT(*) FROM forum.topic_tags tt WHERE tt.tag_id = t.id) AS topic_count
             FROM forum.tags t ORDER BY t.name",
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn create(dto: CreateTagDto, db: &PgPool) -> Result<Tag> {
        let slug = slugify(&dto.name);
        sqlx::query_as::<_, Tag>(
            "INSERT INTO forum.tags (name, slug, color) VALUES ($1, $2, $3)
             RETURNING *, 0::bigint AS topic_count",
        )
        .bind(dto.name.trim())
        .bind(&slug)
        .bind(dto.color.as_deref().unwrap_or("#0d9488"))
        .fetch_one(db)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.is_unique_violation() => {
                ForumError::Conflict("a tag with this name already exists".into())
            }
            other => ForumError::Database(other),
        })
    }

    pub async fn delete(id: Uuid, db: &PgPool) -> Result<()> {
        let r = sqlx::query("DELETE FROM forum.tags WHERE id = $1").bind(id).execute(db).await?;
        if r.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("Tag {id}")));
        }
        Ok(())
    }

    pub async fn for_topic(topic_id: Uuid, db: &PgPool) -> Result<Vec<Tag>> {
        let rows = sqlx::query_as::<_, Tag>(
            "SELECT t.*, 0::bigint AS topic_count FROM forum.tags t
             JOIN forum.topic_tags tt ON tt.tag_id = t.id
             WHERE tt.topic_id = $1 ORDER BY t.name",
        )
        .bind(topic_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Tags for many topics at once (forum listing). Returns topic_id → tags.
    pub async fn for_topics(topic_ids: &[Uuid], db: &PgPool) -> Result<HashMap<Uuid, Vec<Tag>>> {
        if topic_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT tt.topic_id, t.id, t.name, t.slug, t.color, t.created_at
             FROM forum.topic_tags tt JOIN forum.tags t ON t.id = tt.tag_id
             WHERE tt.topic_id = ANY($1)",
        )
        .bind(topic_ids)
        .fetch_all(db)
        .await?;
        let mut map: HashMap<Uuid, Vec<Tag>> = HashMap::new();
        for (topic_id, id, name, slug, color, created_at) in rows {
            map.entry(topic_id).or_default().push(Tag { id, name, slug, color, topic_count: 0, created_at });
        }
        Ok(map)
    }

    /// Replaces a topic's tags (moderator/author action).
    pub async fn set_topic_tags(topic_id: Uuid, tag_ids: &[Uuid], db: &PgPool) -> Result<Vec<Tag>> {
        let mut tx = db.begin().await?;
        sqlx::query("DELETE FROM forum.topic_tags WHERE topic_id = $1")
            .bind(topic_id)
            .execute(&mut *tx)
            .await?;
        for tag_id in tag_ids {
            sqlx::query(
                "INSERT INTO forum.topic_tags (topic_id, tag_id)
                 SELECT $1, $2 WHERE EXISTS (SELECT 1 FROM forum.tags WHERE id = $2)
                 ON CONFLICT DO NOTHING",
            )
            .bind(topic_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Self::for_topic(topic_id, db).await
    }
}
