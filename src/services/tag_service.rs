use std::collections::HashMap;

use kubuno_db::dialect::SqlType;
use kubuno_db::{new_id, params, DbPool, DbQueryBuilder};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::tag::{CreateTagDto, Tag},
    services::aggregates::slugify,
};

pub struct TagService;

impl TagService {
    pub async fn list(db: &DbPool) -> Result<Vec<Tag>> {
        let rows = db
            .fetch_all_as::<Tag>(
                "SELECT t.*, (SELECT COUNT(*) FROM forum.topic_tags tt WHERE tt.tag_id = t.id) AS topic_count \
                 FROM forum.tags t ORDER BY t.name",
                params![],
            )
            .await?;
        Ok(rows)
    }

    pub async fn create(dto: CreateTagDto, db: &DbPool) -> Result<Tag> {
        let slug = slugify(&dto.name);
        let id = new_id();
        db.execute(
            "INSERT INTO forum.tags (id, name, slug, color) VALUES ($1, $2, $3, $4)",
            params![
                id,
                dto.name.trim(),
                slug,
                dto.color.as_deref().unwrap_or("#0d9488")
            ],
        )
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.is_unique_violation() => {
                ForumError::Conflict("a tag with this name already exists".into())
            }
            other => ForumError::Database(other),
        })?;
        db.fetch_one_as::<Tag>(
            "SELECT t.*, (SELECT COUNT(*) FROM forum.topic_tags tt WHERE tt.tag_id = t.id) AS topic_count \
             FROM forum.tags t WHERE t.id = $1",
            params![id],
        )
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, db: &DbPool) -> Result<()> {
        let affected = db
            .execute("DELETE FROM forum.tags WHERE id = $1", params![id])
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Tag {id}")));
        }
        Ok(())
    }

    pub async fn for_topic(topic_id: Uuid, db: &DbPool) -> Result<Vec<Tag>> {
        let zero = db.backend().cast("0", SqlType::BigInt);
        let sql = format!(
            "SELECT t.*, {zero} AS topic_count FROM forum.tags t \
             JOIN forum.topic_tags tt ON tt.tag_id = t.id \
             WHERE tt.topic_id = $1 ORDER BY t.name"
        );
        let rows = db.fetch_all_as::<Tag>(&sql, params![topic_id]).await?;
        Ok(rows)
    }

    /// Tags for many topics at once (forum listing). Returns topic_id → tags.
    pub async fn for_topics(topic_ids: &[Uuid], db: &DbPool) -> Result<HashMap<Uuid, Vec<Tag>>> {
        if topic_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "SELECT tt.topic_id, t.id, t.name, t.slug, t.color, t.created_at \
             FROM forum.topic_tags tt JOIN forum.tags t ON t.id = tt.tag_id \
             WHERE tt.topic_id",
        );
        qb.push_in(topic_ids.iter().copied());
        let rows: Vec<(Uuid, Uuid, String, String, String, chrono::DateTime<chrono::Utc>)> =
            qb.fetch_all_as(db).await?;
        let mut map: HashMap<Uuid, Vec<Tag>> = HashMap::new();
        for (topic_id, id, name, slug, color, created_at) in rows {
            map.entry(topic_id).or_default().push(Tag { id, name, slug, color, topic_count: 0, created_at });
        }
        Ok(map)
    }

    /// Replaces a topic's tags (moderator/author action).
    pub async fn set_topic_tags(topic_id: Uuid, tag_ids: &[Uuid], db: &DbPool) -> Result<Vec<Tag>> {
        let b = db.backend();
        // A guarded, conflict-tolerant insert: only tags that exist are linked,
        // and a re-link is a no-op.
        // A placeholder is never reused on the portable path, so `tag_id` is
        // bound again for the EXISTS guard ($3).
        let insert_sql = format!(
            "INSERT {}INTO forum.topic_tags (topic_id, tag_id) \
             SELECT $1, $2 WHERE EXISTS (SELECT {} FROM forum.tags WHERE id = $3){}",
            b.insert_ignore_prefix(),
            b.cast("1", SqlType::BigInt),
            b.on_conflict_do_nothing(&["topic_id", "tag_id"]),
        );

        let mut tx = db.begin().await?;
        tx.execute(
            "DELETE FROM forum.topic_tags WHERE topic_id = $1",
            params![topic_id],
        )
        .await?;
        for tag_id in tag_ids {
            tx.execute(&insert_sql, params![topic_id, *tag_id, *tag_id]).await?;
        }
        tx.commit().await?;
        Self::for_topic(topic_id, db).await
    }
}
